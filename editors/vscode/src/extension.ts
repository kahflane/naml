import * as path from 'path';
import * as fs from 'fs';
import { workspace, ExtensionContext, window } from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

function findServerBinary(context: ExtensionContext): string | undefined {
  const config = workspace.getConfiguration('naml');
  const configPath = config.get<string>('lsp.path');

  if (configPath && configPath.length > 0) {
    if (fs.existsSync(configPath)) {
      return configPath;
    }
    window.showWarningMessage(`Configured naml-lsp path not found: ${configPath}`);
  }

  const ext = process.platform === 'win32' ? '.exe' : '';
  const binaryName = `naml-lsp${ext}`;

  const bundledPath = context.asAbsolutePath(path.join('server', binaryName));
  if (fs.existsSync(bundledPath)) {
    return bundledPath;
  }

  const envPath = process.env.PATH;
  if (envPath) {
    const pathDirs = envPath.split(path.delimiter);
    for (const dir of pathDirs) {
      const fullPath = path.join(dir, binaryName);
      if (fs.existsSync(fullPath)) {
        return fullPath;
      }
    }
  }

  return undefined;
}

export function activate(context: ExtensionContext) {
  const serverPath = findServerBinary(context);

  if (!serverPath) {
    window.showWarningMessage(
      'naml-lsp binary not found. Syntax highlighting will work, but IntelliSense features are disabled. ' +
      'Build naml-lsp with `cargo build --release -p naml-lsp` and add to PATH, or configure naml.lsp.path.'
    );
    return;
  }

  const serverExecutable: Executable = {
    command: serverPath,
    args: [],
  };

  const serverOptions: ServerOptions = {
    run: serverExecutable,
    debug: serverExecutable
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'naml' }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher('**/*.naml')
    },
    outputChannelName: 'naml Language Server'
  };

  client = new LanguageClient(
    'namlLanguageServer',
    'naml Language Server',
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
