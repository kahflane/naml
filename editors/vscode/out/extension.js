"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
const vscode_1 = require("vscode");
const node_1 = require("vscode-languageclient/node");
let client;
function findServerBinary(context) {
    const config = vscode_1.workspace.getConfiguration('naml');
    const configPath = config.get('lsp.path');
    if (configPath && configPath.length > 0) {
        if (fs.existsSync(configPath)) {
            return configPath;
        }
        vscode_1.window.showWarningMessage(`Configured naml-lsp path not found: ${configPath}`);
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
function activate(context) {
    const serverPath = findServerBinary(context);
    if (!serverPath) {
        vscode_1.window.showWarningMessage('naml-lsp binary not found. Syntax highlighting will work, but IntelliSense features are disabled. ' +
            'Build naml-lsp with `cargo build --release -p naml-lsp` and add to PATH, or configure naml.lsp.path.');
        return;
    }
    const serverExecutable = {
        command: serverPath,
        args: [],
    };
    const serverOptions = {
        run: serverExecutable,
        debug: serverExecutable
    };
    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'naml' }],
        synchronize: {
            fileEvents: vscode_1.workspace.createFileSystemWatcher('**/*.naml')
        },
        outputChannelName: 'naml Language Server'
    };
    client = new node_1.LanguageClient('namlLanguageServer', 'naml Language Server', serverOptions, clientOptions);
    client.start();
}
function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
//# sourceMappingURL=extension.js.map