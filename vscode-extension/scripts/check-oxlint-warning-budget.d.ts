export interface OxlintDiagnostic {
  severity?: string;
  code?: string;
  filename?: string;
  message?: string;
  labels?: Array<{ span?: { line?: number; column?: number } }>;
}

export type CountMap = Record<string, number>;

export interface OxlintWarningInventory {
  warning_count: number;
  by_rule: CountMap;
  by_surface: CountMap;
  by_rule_and_surface: Record<string, CountMap>;
  by_file: CountMap;
}

export function buildInventory(diagnostics: OxlintDiagnostic[]): OxlintWarningInventory;
export function compareInventory(
  current: OxlintWarningInventory,
  baseline: OxlintWarningInventory,
): string[];
export function failureExitCode(errors: OxlintDiagnostic[], status: number | null): number;
export function surfaceForFile(filename: string): string;
