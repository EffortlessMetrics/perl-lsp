import type * as vscode from 'vscode';
import { registerDocumentFeatureGroup, type DocumentFeatureContext } from '../documentFeatureGroup';

function disposable(): vscode.Disposable {
  return { dispose: jest.fn() } as unknown as vscode.Disposable;
}

function makeDependencies(): DocumentFeatureContext & {
  registerGherkinProviders: jest.Mock;
  registerGherkinStepDefinitionSupport: jest.Mock;
  registerPodPreview: jest.Mock;
} {
  return {
    extensionContext: {} as vscode.ExtensionContext,
    registerGherkinProviders: jest.fn(() => [disposable()]),
    registerGherkinStepDefinitionSupport: jest.fn(() => [disposable()]),
    registerPodPreview: jest.fn(() => [disposable()]),
  };
}

describe('registerDocumentFeatureGroup', () => {
  test('composes all document providers in stable registration order', () => {
    const dependencies = makeDependencies();

    const disposables = registerDocumentFeatureGroup(dependencies);

    expect(disposables).toHaveLength(3);
    expect(dependencies.registerGherkinProviders).toHaveBeenCalledTimes(1);
    expect(dependencies.registerGherkinStepDefinitionSupport).toHaveBeenCalledTimes(1);
    expect(dependencies.registerPodPreview).toHaveBeenCalledWith(dependencies.extensionContext);
  });

  test('does not register providers until the group is composed', () => {
    const dependencies = makeDependencies();

    expect(dependencies.registerGherkinProviders).not.toHaveBeenCalled();
    expect(dependencies.registerGherkinStepDefinitionSupport).not.toHaveBeenCalled();
    expect(dependencies.registerPodPreview).not.toHaveBeenCalled();

    registerDocumentFeatureGroup(dependencies);

    expect(dependencies.registerGherkinProviders).toHaveBeenCalledTimes(1);
  });
});
