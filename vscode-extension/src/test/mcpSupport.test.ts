import { formatMcpUnavailableMessage } from '../mcpSupport';

describe('formatMcpUnavailableMessage', () => {
  test('explains how to recover when no MCP servers are configured', () => {
    const message = formatMcpUnavailableMessage(0);
    expect(message).toContain('VS Code MCP API unavailable');
    expect(message).toContain('Update to a VS Code build with MCP support');
    expect(message).toContain('perl-lsp.mcp.servers');
    expect(message).not.toContain('will not be published');
  });

  test('mentions one configured server that will not be published', () => {
    const message = formatMcpUnavailableMessage(1);
    expect(message).toContain('1 configured MCP server will not be published');
  });

  test('pluralizes multiple configured servers', () => {
    const message = formatMcpUnavailableMessage(2);
    expect(message).toContain('2 configured MCP servers will not be published');
  });
});
