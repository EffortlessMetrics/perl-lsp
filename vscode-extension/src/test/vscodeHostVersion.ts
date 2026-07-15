const MINIMUM_SUPPORTED_VSCODE_VERSION = '1.125.0';

export const MINIMUM_SUPPORTED_VSCODE_VERSION_REQUEST = MINIMUM_SUPPORTED_VSCODE_VERSION;

function versionParts(version: string): [number, number, number] {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(version);
  if (!match) {
    throw new Error(
      `VS Code test version must be stable, insiders, or an exact major.minor.patch version; got ${version}`,
    );
  }
  const major = match[1];
  const minor = match[2];
  const patch = match[3];
  if (major === undefined || minor === undefined || patch === undefined) {
    throw new Error(`VS Code version match was incomplete: ${version}`);
  }
  return [Number(major), Number(minor), Number(patch)];
}

function compareVersions(left: [number, number, number], right: [number, number, number]): number {
  for (let index = 0; index < left.length; index += 1) {
    const difference = (left[index] ?? 0) - (right[index] ?? 0);
    if (difference !== 0) {
      return difference;
    }
  }
  return 0;
}

export function resolveVSCodeTestVersion(rawVersion: string | undefined): string {
  const version = rawVersion?.trim() || 'stable';
  if (version === 'stable' || version === 'insiders') {
    return version;
  }

  const parts = versionParts(version);
  const minimum = versionParts(MINIMUM_SUPPORTED_VSCODE_VERSION);
  if (compareVersions(parts, minimum) < 0) {
    throw new Error(
      `VS Code test version ${version} is below the declared minimum ${MINIMUM_SUPPORTED_VSCODE_VERSION}`,
    );
  }
  return version;
}
