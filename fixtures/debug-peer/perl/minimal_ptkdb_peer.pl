#!/usr/bin/env perl
#
# minimal_ptkdb_peer.pl -- a tiny, heavily-commented REFERENCE peer for
# perl-dap's "Perl Debugger Peer Protocol" (perl-debug-peer-v1).
#
# THIS IS NOT PRODUCTION Devel::ptkdb CODE. It is the smallest possible
# program that speaks the wire protocol correctly, meant to be read
# top-to-bottom and adapted by whoever writes the real ptkdb-side patch
# (see docs/reference/PTKDB_PEER_INTEGRATION_TARGET.md, "## Minimum
# upstream ptkdb PR").
#
# What it does, in order:
#   1. Reads the rendezvous env vars perl-dap sets before launching a peer:
#        PERL_DAP_PEER       "HOST:PORT" perl-dap is listening on
#        PERL_DAP_PEER_TOKEN per-session shared secret (optional)
#        PERL_DAP_PEER_MODE  control mode; this reference only speaks "mirror"
#   2. Opens a TCP socket back to perl-dap.
#   3. Sends one "peer/hello" request (the handshake) and reads the response.
#   4. Emits one "debugger/output" event and one "debugger/stopped" event --
#      the two events every mirror-mode peer needs to produce to be useful.
#   5. Waits for perl-dap to close the connection (or a read error/EOF) and
#      exits cleanly.
#
# It deliberately does NOT implement: continue/next/stepIn/stepOut/pause,
# setBreakpoints, stackTrace, scopes, variables, or evaluate. A real ptkdb
# integration can add those later by answering the corresponding
# peer-protocol requests (see the "capabilities" object in peer/hello --
# every capability defaults to false, so a peer that only reports stops is a
# complete, honest v1).
#
# Wire format (verified against crates/perl-dap/src/peer_protocol/ in the
# perl-lsp-swarm repo, decision D4: reuses the LSP base-protocol framing):
#   Content-Length: <N>\r\n
#   \r\n
#   <N bytes of UTF-8 JSON>
#
# No Content-Type header, no trailing newline after the body. Framing is
# byte-for-byte the same as LSP/DAP base protocol, so any LSP/DAP codec you
# already have lying around also works here.
#
# Message envelope (crates/perl-dap/src/peer_protocol/message.rs):
#   Request:  {"type":"request", "seq":N, "command":"...", "arguments":{...}}
#   Response: {"type":"response","seq":N,"requestSeq":N,"success":bool,
#              "command":"...","message":"...","body":{...}}
#   Event:    {"type":"event",   "seq":N, "event":"...",  "body":{...}}
#
# Only core Perl modules are used: IO::Socket::INET (network) and JSON::PP
# (JSON, core since Perl 5.14).

use strict;
use warnings;
use IO::Socket::INET;
use JSON::PP qw(encode_json decode_json);

# The exact protocol version string the host checks in peer/hello. Pinned
# here as a literal because this script has no access to the Rust crate;
# see crates/perl-dap/src/peer_protocol/mod.rs PROTOCOL_VERSION.
use constant PROTOCOL_VERSION => 'perl-debug-peer-v1';

# --- 1. Read the rendezvous env contract -----------------------------------

my $peer_addr = $ENV{PERL_DAP_PEER}
    or die "minimal_ptkdb_peer.pl: PERL_DAP_PEER not set (nothing to connect to)\n";
my $token = $ENV{PERL_DAP_PEER_TOKEN};   # optional -- undef when the host minted none
my $mode  = $ENV{PERL_DAP_PEER_MODE} // '';

# This reference peer only knows how to be a mirror-mode reporter. A real
# integration should do the same until cooperative/dapControlled modes ship:
# ignore (exit 0, do nothing) rather than guess at unsupported behavior.
if ($mode ne 'mirror') {
    print STDERR "minimal_ptkdb_peer.pl: PERL_DAP_PEER_MODE='$mode' is not 'mirror' -- nothing to do, exiting\n";
    exit 0;
}

my ($host, $port) = split /:/, $peer_addr, 2;
die "minimal_ptkdb_peer.pl: malformed PERL_DAP_PEER '$peer_addr' (want HOST:PORT)\n"
    unless defined $host && defined $port && length $host && $port =~ /^\d+\z/;

# --- 2. Connect back to perl-dap --------------------------------------------

my $sock = IO::Socket::INET->new(
    PeerAddr => $host,
    PeerPort => $port,
    Proto    => 'tcp',
) or die "minimal_ptkdb_peer.pl: cannot connect to $peer_addr: $!\n";
$sock->autoflush(1);

my $seq = 0;
sub next_seq { return ++$seq }

# --- Framing helpers (Content-Length, matches perl_lsp_rs_core::transport) --

sub send_message {
    my ($msg) = @_;
    my $body = encode_json($msg);
    print {$sock} "Content-Length: " . length($body) . "\r\n\r\n" . $body
        or die "minimal_ptkdb_peer.pl: write failed: $!\n";
}

# Reads exactly one framed message, or returns undef on clean EOF.
sub read_message {
    my $header = '';
    while ($header !~ /\r\n\r\n\z/) {
        my $byte;
        my $n = sysread($sock, $byte, 1);
        return undef if !defined $n || $n == 0;   # EOF / connection closed
        $header .= $byte;
    }
    my ($len) = $header =~ /Content-Length:\s*(\d+)/i
        or die "minimal_ptkdb_peer.pl: frame missing Content-Length header\n";
    my $body = '';
    while (length($body) < $len) {
        my $chunk;
        my $n = sysread($sock, $chunk, $len - length($body));
        return undef if !defined $n || $n == 0;
        $body .= $chunk;
    }
    return decode_json($body);
}

# --- 3. Handshake: send peer/hello, read the host's response ---------------

my %hello_args = (
    peer            => 'minimal_ptkdb_peer.pl (reference)',
    peerVersion     => '0.1',
    protocolVersion => PROTOCOL_VERSION,
    # Every field defaults to false/absent on the host side, so an empty
    # capabilities object is a complete, honest report for a peer that only
    # ever emits output + stopped.
    capabilities    => {},
);
$hello_args{token} = $token if defined $token;   # omit entirely when unset

send_message({
    type      => 'request',
    seq       => next_seq(),
    command   => 'peer/hello',
    arguments => \%hello_args,
});

my $hello_resp = read_message();
if (!defined $hello_resp) {
    die "minimal_ptkdb_peer.pl: connection closed before peer/hello response\n";
}
if (!$hello_resp->{success}) {
    my $why = $hello_resp->{message} // 'unknown reason';
    die "minimal_ptkdb_peer.pl: peer/hello rejected by host: $why\n";
}
my $session_id = $hello_resp->{body}{sessionId} // '(no sessionId)';
print STDERR "minimal_ptkdb_peer.pl: handshake ok, session=$session_id\n";

# --- 4. Emit one output event, then one stopped event -----------------------

send_message({
    type  => 'event',
    seq   => next_seq(),
    event => 'debugger/output',
    body  => {
        category => 'stdout',
        output   => "minimal_ptkdb_peer.pl: reference peer connected\n",
    },
});

send_message({
    type  => 'event',
    seq   => next_seq(),
    event => 'debugger/stopped',
    body  => {
        reason   => 'entry',
        threadId => 1,
        source   => { path => $0 },
        line     => 1,
    },
});

# --- 5. Wait for the host to end the session, then exit cleanly ------------
#
# A real ptkdb integration would loop here, driving further output/stopped
# events as the Tk UI steps through the program, and answering any
# stack/scopes/variables/evaluate requests it chooses to support. This
# reference has nothing further to report, so it just blocks on the socket
# until perl-dap closes it (clean EOF) and exits 0.
while (1) {
    my $msg = read_message();
    last if !defined $msg;   # host closed the connection -- exit cleanly
    # An unsolicited host->peer request (e.g. debugger/setBreakpoints) would
    # arrive here in a real integration; this reference peer has no control
    # capabilities to answer with, so it just keeps waiting.
}

close($sock);
exit 0;
