import type * as vscode from 'vscode';

/**
 * Explicit composition dependencies for document-language providers.
 *
 * Registration stays separate from activation timing: the entry module owns
 * the feature metric, while this group owns the POD/Gherkin provider set.
 */
export interface DocumentFeatureContext {
  readonly extensionContext: vscode.ExtensionContext;
  readonly registerGherkinProviders: () => vscode.Disposable[];
  readonly registerGherkinStepDefinitionSupport: () => vscode.Disposable[];
  readonly registerPodPreview: (context: vscode.ExtensionContext) => vscode.Disposable[];
}

/** Register the POD and Gherkin providers owned by the document feature group. */
export function registerDocumentFeatureGroup(
  dependencies: DocumentFeatureContext,
): vscode.Disposable[] {
  return [
    ...dependencies.registerGherkinProviders(),
    ...dependencies.registerGherkinStepDefinitionSupport(),
    ...dependencies.registerPodPreview(dependencies.extensionContext),
  ];
}
