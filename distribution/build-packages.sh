#!/bin/bash
# Build distribution packages for perl-lsp

set -e

VERSION="1.0.0"
PACKAGE_NAME="perl-lsp"
DESCRIPTION="High-performance Perl Language Server with 100% syntax coverage"
MAINTAINER="Steven Zimmerman, CPA <git@effortlesssteven.com>"
HOMEPAGE="https://github.com/EffortlessMetrics/perl-lsp"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building perl-lsp distribution packages v${VERSION}${NC}"

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo is not installed${NC}"
    exit 1
fi

# Build the release binary
echo -e "${YELLOW}Building release binary...${NC}"
cargo build --release --bin perl-lsp -p perl-parser

# Create temporary directory for packaging
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Prepare binary
BIN_DIR="$TEMP_DIR/usr/bin"
mkdir -p "$BIN_DIR"
cp target/release/perl-lsp "$BIN_DIR/"
chmod 755 "$BIN_DIR/perl-lsp"

# Create man page
MAN_DIR="$TEMP_DIR/usr/share/man/man1"
mkdir -p "$MAN_DIR"
cat > "$MAN_DIR/perl-lsp.1" << 'EOF'
.TH PERL-LSP 1 "February 2026" "perl-lsp 1.0.0" "User Commands"
.SH NAME
perl-lsp \- Perl Language Server Protocol implementation
.SH SYNOPSIS
.B perl-lsp
[\fB\-\-stdio\fR]
[\fB\-\-tcp\fR \fIPORT\fR]
[\fB\-\-log\fR]
[\fB\-\-version\fR]
[\fB\-\-help\fR]
.SH DESCRIPTION
.B perl-lsp
is a high-performance Language Server Protocol implementation for Perl,
providing IDE features like code completion, go-to-definition, find references,
and more.
.SH OPTIONS
.TP
.B \-\-stdio
Communicate via standard input/output (default)
.TP
.B \-\-tcp PORT
Listen on TCP port PORT
.TP
.B \-\-log
Enable debug logging
.TP
.B \-\-version
Display version information
.TP
.B \-\-help
Display help message
.SH EXAMPLES
.TP
Start the server for editor integration:
.B perl-lsp --stdio
.TP
Start with debug logging:
.B perl-lsp --stdio --log
.SH AUTHOR
Steven Zimmerman, CPA
.SH BUGS
Report bugs at https://github.com/EffortlessMetrics/perl-lsp/issues
EOF
gzip -9 "$MAN_DIR/perl-lsp.1"

# Create systemd service file (optional)
SYSTEMD_DIR="$TEMP_DIR/usr/lib/systemd/user"
mkdir -p "$SYSTEMD_DIR"
cat > "$SYSTEMD_DIR/perl-lsp.service" << EOF
[Unit]
Description=Perl Language Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/perl-lsp --tcp 9999
Restart=on-failure
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
EOF

# Function to build DEB package
build_deb() {
    echo -e "${YELLOW}Building .deb package...${NC}"
    
    if ! command -v dpkg-deb &> /dev/null; then
        echo -e "${RED}Warning: dpkg-deb not found, skipping .deb build${NC}"
        return
    fi
    
    DEB_DIR="$TEMP_DIR/DEBIAN"
    mkdir -p "$DEB_DIR"
    
    # Create control file
    cat > "$DEB_DIR/control" << EOF
Package: ${PACKAGE_NAME}
Version: ${VERSION}
Section: devel
Priority: optional
Architecture: amd64
Maintainer: ${MAINTAINER}
Description: ${DESCRIPTION}
 perl-lsp provides a Language Server Protocol implementation for Perl,
 offering advanced IDE features including:
 - Real-time syntax checking
 - Code completion
 - Go-to-definition
 - Find references
 - Symbol search
 - Refactoring support
 - 100% Perl syntax coverage
Homepage: ${HOMEPAGE}
EOF
    
    # Create postinst script
    cat > "$DEB_DIR/postinst" << 'EOF'
#!/bin/sh
set -e

if [ "$1" = "configure" ]; then
    echo "perl-lsp has been installed successfully."
    echo "To use with your editor, configure it to use 'perl-lsp --stdio'"
fi

exit 0
EOF
    chmod 755 "$DEB_DIR/postinst"
    
    # Build the package
    dpkg-deb --build "$TEMP_DIR" "perl-lsp_${VERSION}_amd64.deb"
    echo -e "${GREEN}Created perl-lsp_${VERSION}_amd64.deb${NC}"
}

# Function to build RPM package
build_rpm() {
    echo -e "${YELLOW}Building .rpm package...${NC}"
    
    if ! command -v rpmbuild &> /dev/null; then
        echo -e "${RED}Warning: rpmbuild not found, skipping .rpm build${NC}"
        return
    fi
    
    # Create RPM build structure
    RPM_BUILD_DIR="$HOME/rpmbuild"
    mkdir -p "$RPM_BUILD_DIR"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
    
    # Create tarball of the binary
    TAR_NAME="perl-lsp-${VERSION}.tar.gz"
    cd "$TEMP_DIR"
    tar czf "$RPM_BUILD_DIR/SOURCES/$TAR_NAME" usr/
    cd -
    
    # Create spec file
    cat > "$RPM_BUILD_DIR/SPECS/perl-lsp.spec" << EOF
Name:           perl-lsp
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        ${DESCRIPTION}
License:        MIT
URL:            ${HOMEPAGE}
Source0:        %{name}-%{version}.tar.gz

%description
perl-lsp provides a Language Server Protocol implementation for Perl,
offering advanced IDE features including real-time syntax checking,
code completion, go-to-definition, find references, symbol search,
refactoring support, and 100% Perl syntax coverage.

%prep
%setup -q -c

%install
rm -rf \$RPM_BUILD_ROOT
mkdir -p \$RPM_BUILD_ROOT
cp -a usr \$RPM_BUILD_ROOT/

%files
%defattr(-,root,root,-)
/usr/bin/perl-lsp
/usr/share/man/man1/perl-lsp.1.gz
/usr/lib/systemd/user/perl-lsp.service

%changelog
* $(date +"%a %b %d %Y") ${MAINTAINER} - ${VERSION}-1
- Initial package release
EOF
    
    # Build the RPM
    rpmbuild -bb "$RPM_BUILD_DIR/SPECS/perl-lsp.spec"
    
    # Copy the built RPM to current directory
    cp "$RPM_BUILD_DIR/RPMS/x86_64/perl-lsp-${VERSION}"*.rpm .
    echo -e "${GREEN}Created perl-lsp-${VERSION}.rpm${NC}"
}

# Function to build tarball
build_tarball() {
    echo -e "${YELLOW}Building .tar.gz archive...${NC}"
    
    ARCHIVE_NAME="perl-lsp-${VERSION}-linux-x86_64.tar.gz"
    
    # Create a clean directory structure
    ARCHIVE_DIR="$TEMP_DIR/perl-lsp-${VERSION}"
    mkdir -p "$ARCHIVE_DIR"
    cp target/release/perl-lsp "$ARCHIVE_DIR/"
    
    # Add README
    cat > "$ARCHIVE_DIR/README.md" << EOF
# Perl Language Server v${VERSION}

## Installation

1. Extract the archive
2. Move perl-lsp to a directory in your PATH:
   \`\`\`bash
   sudo cp perl-lsp /usr/local/bin/
   chmod +x /usr/local/bin/perl-lsp
   \`\`\`

## Usage

Configure your editor to use \`perl-lsp --stdio\`

## Documentation

See https://github.com/EffortlessMetrics/perl-lsp
EOF
    
    # Create the archive
    cd "$TEMP_DIR"
    tar czf "$OLDPWD/$ARCHIVE_NAME" "perl-lsp-${VERSION}/"
    cd -
    
    echo -e "${GREEN}Created $ARCHIVE_NAME${NC}"
}

# Build all packages
build_deb
build_rpm
build_tarball

echo -e "${GREEN}All packages built successfully!${NC}"
echo -e "${YELLOW}Files created:${NC}"
ls -lh perl-lsp*.{deb,rpm,tar.gz} 2>/dev/null || true