import path from "node:path";
import process from "node:process";
import ts from "typescript";

const IMPL_FILE =
  "src/features/git-history/components/git-history-panel/components/GitHistoryPanelImpl.tsx";
const VIEW_FILE =
  "src/features/git-history/components/git-history-panel/components/GitHistoryPanelView.tsx";
const DIALOGS_FILE =
  "src/features/git-history/components/git-history-panel/components/GitHistoryPanelDialogs.tsx";
const HOOK_FILE =
  "src/features/git-history/components/git-history-panel/hooks/useGitHistoryPanelInteractions.tsx";

const CONTRACT_FILES = [IMPL_FILE, VIEW_FILE, DIALOGS_FILE, HOOK_FILE];

const ALLOWED_GLOBALS = new Set([
  "globalThis",
  "window",
  "document",
  "console",
  "setTimeout",
  "clearTimeout",
  "setInterval",
  "clearInterval",
  "requestAnimationFrame",
  "cancelAnimationFrame",
  "queueMicrotask",
  "Promise",
  "Math",
  "Number",
  "String",
  "Boolean",
  "Array",
  "Object",
  "JSON",
  "Date",
  "RegExp",
  "Error",
  "TypeError",
  "URL",
  "URLSearchParams",
  "Intl",
  "Map",
  "Set",
  "WeakMap",
  "WeakSet",
  "Symbol",
  "BigInt",
  "parseInt",
  "parseFloat",
  "isNaN",
  "isFinite",
  "encodeURIComponent",
  "decodeURIComponent",
  "encodeURI",
  "decodeURI",
  "navigator",
  "location",
  "history",
  "MutationObserver",
  "IntersectionObserver",
  "ResizeObserver",
  "HTMLElement",
  "Node",
  "Element",
  "Event",
  "CustomEvent",
  "AbortController",
  "AbortSignal",
  "fetch",
  "performance",
  "localStorage",
  "sessionStorage",
  "crypto",
  "btoa",
  "atob",
  "process",
  "Buffer",
  "module",
  "require",
  "__dirname",
  "__filename",
  "undefined",
  "alert",
  "confirm",
  "prompt",
]);

function toAbsolutePath(relativePath) {
  return path.resolve(process.cwd(), relativePath);
}

function getProgramSourceFile(program, relativePath) {
  const sourceFile = program.getSourceFile(toAbsolutePath(relativePath));
  if (!sourceFile) {
    throw new Error(`Cannot load source file "${relativePath}".`);
  }
  return sourceFile;
}

function visitNode(node, callback) {
  callback(node);
  node.forEachChild((child) => visitNode(child, callback));
}

function getPropertyNameText(name) {
  if (!name) {
    return null;
  }
  if (
    ts.isIdentifier(name) ||
    ts.isStringLiteral(name) ||
    ts.isNumericLiteral(name)
  ) {
    return name.text;
  }
  return null;
}

function collectObjectLiteralOwnKeys(objectLiteral) {
  const keys = new Set();
  for (const property of objectLiteral.properties) {
    if (ts.isShorthandPropertyAssignment(property)) {
      keys.add(property.name.text);
      continue;
    }
    if (
      ts.isPropertyAssignment(property) ||
      ts.isMethodDeclaration(property) ||
      ts.isGetAccessorDeclaration(property) ||
      ts.isSetAccessorDeclaration(property)
    ) {
      const key = getPropertyNameText(property.name);
      if (key) {
        keys.add(key);
      }
    }
  }
  return keys;
}

function findFunctionDeclaration(sourceFile, functionName) {
  for (const statement of sourceFile.statements) {
    if (
      ts.isFunctionDeclaration(statement) &&
      statement.name?.text === functionName
    ) {
      if (!statement.body) {
        throw new Error(`Function "${functionName}" has no body.`);
      }
      return statement;
    }
  }
  throw new Error(`Cannot find function "${functionName}" in ${sourceFile.fileName}.`);
}

function isTypePosition(node) {
  let current = node.parent;
  while (current) {
    if (ts.isTypeNode(current) || ts.isExpressionWithTypeArguments(current)) {
      return true;
    }
    current = current.parent;
  }
  return false;
}

function isDeclarationIdentifier(node) {
  const parent = node.parent;
  if (!parent) {
    return false;
  }
  if (ts.isBindingElement(parent) && (parent.name === node || parent.propertyName === node)) {
    return true;
  }
  if (ts.isVariableDeclaration(parent) && parent.name === node) {
    return true;
  }
  if (ts.isParameter(parent) && parent.name === node) {
    return true;
  }
  if (ts.isFunctionDeclaration(parent) && parent.name === node) {
    return true;
  }
  if (ts.isFunctionExpression(parent) && parent.name === node) {
    return true;
  }
  if (ts.isClassDeclaration(parent) && parent.name === node) {
    return true;
  }
  if (ts.isImportClause(parent) && parent.name === node) {
    return true;
  }
  if (ts.isImportSpecifier(parent) && (parent.name === node || parent.propertyName === node)) {
    return true;
  }
  if (ts.isNamespaceImport(parent) && parent.name === node) {
    return true;
  }
  if (ts.isPropertyAssignment(parent) && parent.name === node) {
    return true;
  }
  if (ts.isMethodDeclaration(parent) && parent.name === node) {
    return true;
  }
  if (ts.isPropertyDeclaration(parent) && parent.name === node) {
    return true;
  }
  if (ts.isGetAccessorDeclaration(parent) && parent.name === node) {
    return true;
  }
  if (ts.isSetAccessorDeclaration(parent) && parent.name === node) {
    return true;
  }
  if (ts.isEnumMember(parent) && parent.name === node) {
    return true;
  }
  return false;
}

function shouldSkipIdentifier(node) {
  const parent = node.parent;
  if (!parent) {
    return false;
  }
  if (ts.isPropertyAccessExpression(parent) && parent.name === node) {
    return true;
  }
  if (ts.isQualifiedName(parent) && parent.right === node) {
    return true;
  }
  if (
    ts.isJsxOpeningElement(parent) ||
    ts.isJsxSelfClosingElement(parent) ||
    ts.isJsxClosingElement(parent)
  ) {
    return true;
  }
  if (ts.isJsxAttribute(parent) && parent.name === node) {
    return true;
  }
  return false;
}

function getNodeLocation(sourceFile, node) {
  const { line, character } = sourceFile.getLineAndCharacterOfPosition(
    node.getStart(sourceFile),
  );
  return `${line + 1}:${character + 1}`;
}

function getScopeDestructureKeys(sourceFile, functionName, checker) {
  const fn = findFunctionDeclaration(sourceFile, functionName);
  const allKeys = new Set();
  const bindingSymbols = new Map();
  const bindingNameNodes = new Set();

  visitNode(fn.body, (node) => {
    if (
      ts.isVariableDeclaration(node) &&
      ts.isObjectBindingPattern(node.name) &&
      node.initializer &&
      ts.isIdentifier(node.initializer) &&
      node.initializer.text === "scope"
    ) {
      for (const element of node.name.elements) {
        if (element.dotDotDotToken) {
          continue;
        }
        if (!ts.isIdentifier(element.name)) {
          continue;
        }
        bindingNameNodes.add(element.name);
        const symbol = checker.getSymbolAtLocation(element.name);
        const key = element.propertyName
          ? getPropertyNameText(element.propertyName)
          : element.name.text;
        if (!key) {
          continue;
        }
        allKeys.add(key);
        if (symbol) {
          bindingSymbols.set(symbol, key);
        }
      }
    }
  });

  if (allKeys.size === 0) {
    throw new Error(`Cannot find "const { ... } = scope" in ${sourceFile.fileName}.`);
  }

  const usedKeys = new Set();
  visitNode(fn.body, (node) => {
    if (!ts.isIdentifier(node)) {
      return;
    }
    if (bindingNameNodes.has(node)) {
      return;
    }
    if (isDeclarationIdentifier(node) || isTypePosition(node)) {
      return;
    }
    const symbol = checker.getSymbolAtLocation(node);
    if (!symbol) {
      return;
    }
    const key = bindingSymbols.get(symbol);
    if (!key) {
      return;
    }
    usedKeys.add(key);
  });

  if (usedKeys.size === 0) {
    return allKeys;
  }
  return usedKeys;
}

function getFirstCallArgument(sourceFile, calleeName) {
  let result = null;
  visitNode(sourceFile, (node) => {
    if (result) {
      return;
    }
    if (!ts.isCallExpression(node)) {
      return;
    }
    if (ts.isIdentifier(node.expression) && node.expression.text === calleeName) {
      result = node.arguments[0] ?? null;
    }
  });
  if (!result) {
    throw new Error(`Cannot find call "${calleeName}(...)".`);
  }
  return result;
}

function resolveProvidedKeysFromArgument(argumentNode, sourceSetsByIdentifier) {
  if (ts.isIdentifier(argumentNode)) {
    const keys = sourceSetsByIdentifier.get(argumentNode.text);
    if (!keys) {
      throw new Error(`Unknown argument source "${argumentNode.text}".`);
    }
    return { keys: new Set(keys), unresolvedSpreads: [] };
  }

  if (!ts.isObjectLiteralExpression(argumentNode)) {
    throw new Error(`Unsupported argument node kind: ${ts.SyntaxKind[argumentNode.kind]}.`);
  }

  const keys = new Set();
  const unresolvedSpreads = [];

  for (const property of argumentNode.properties) {
    if (ts.isSpreadAssignment(property)) {
      const spreadExpr = property.expression;
      if (ts.isIdentifier(spreadExpr)) {
        const spreadKeys = sourceSetsByIdentifier.get(spreadExpr.text);
        if (!spreadKeys) {
          unresolvedSpreads.push(spreadExpr.text);
          continue;
        }
        for (const key of spreadKeys) {
          keys.add(key);
        }
        continue;
      }
      if (ts.isObjectLiteralExpression(spreadExpr)) {
        const nested = resolveProvidedKeysFromArgument(
          spreadExpr,
          sourceSetsByIdentifier,
        );
        for (const key of nested.keys) {
          keys.add(key);
        }
        unresolvedSpreads.push(...nested.unresolvedSpreads);
        continue;
      }
      unresolvedSpreads.push(spreadExpr.getText());
      continue;
    }

    if (ts.isShorthandPropertyAssignment(property)) {
      keys.add(property.name.text);
      continue;
    }

    if (
      ts.isPropertyAssignment(property) ||
      ts.isMethodDeclaration(property) ||
      ts.isGetAccessorDeclaration(property) ||
      ts.isSetAccessorDeclaration(property)
    ) {
      const key = getPropertyNameText(property.name);
      if (key) {
        keys.add(key);
      }
    }
  }

  return { keys, unresolvedSpreads };
}

function sorted(items) {
  return [...items].sort((a, b) => a.localeCompare(b));
}

function checkContract({ name, requiredKeys, providedKeys, unresolvedSpreads }) {
  const issues = [];
  if (unresolvedSpreads.length > 0) {
    issues.push(`[${name}] unresolved spread source(s): ${sorted(unresolvedSpreads).join(", ")}`);
  }
  const missingKeys = sorted(
    [...requiredKeys].filter((key) => !providedKeys.has(key)),
  );
  if (missingKeys.length > 0) {
    issues.push(`[${name}] missing ${missingKeys.length} key(s): ${missingKeys.join(", ")}`);
  }
  return issues;
}

function checkShorthandBindings(sourceFile, argumentNode, label, checker) {
  if (!ts.isObjectLiteralExpression(argumentNode)) {
    return [];
  }
  const issues = [];
  for (const property of argumentNode.properties) {
    if (!ts.isShorthandPropertyAssignment(property)) {
      continue;
    }
    const name = property.name.text;
    const symbol = checker.getSymbolAtLocation(property.name);
    if (!symbol) {
      issues.push(`[${label}] shorthand "${name}" has no resolvable symbol.`);
      continue;
    }
    const declarations = symbol.declarations ?? [];
    const hasLocalDeclaration = declarations.some(
      (declaration) => declaration.getSourceFile().fileName === sourceFile.fileName,
    );
    if (!hasLocalDeclaration) {
      issues.push(
        `[${label}] shorthand "${name}" resolves to a non-local symbol (likely global).`,
      );
    }
  }
  return issues;
}

function checkRuntimeIdentifierSafety(sourceFile, functionName, checker) {
  const fn = findFunctionDeclaration(sourceFile, functionName);
  const issues = [];
  const seen = new Set();

  visitNode(fn.body, (node) => {
    if (!ts.isIdentifier(node)) {
      return;
    }
    if (isDeclarationIdentifier(node) || isTypePosition(node) || shouldSkipIdentifier(node)) {
      return;
    }
    const name = node.text;
    const symbol = checker.getSymbolAtLocation(node);

    if (!symbol) {
      const key = `${name}:${getNodeLocation(sourceFile, node)}`;
      if (!seen.has(key)) {
        seen.add(key);
        issues.push(
          `[${functionName}] unresolved identifier "${name}" at ${key.split(":").slice(1).join(":")}.`,
        );
      }
      return;
    }

    const declarations = symbol.declarations ?? [];
    const hasLocalDeclaration = declarations.some(
      (declaration) => declaration.getSourceFile().fileName === sourceFile.fileName,
    );
    if (!hasLocalDeclaration && !ALLOWED_GLOBALS.has(name)) {
      const key = `${name}:${getNodeLocation(sourceFile, node)}`;
      if (!seen.has(key)) {
        seen.add(key);
        issues.push(
          `[${functionName}] identifier "${name}" at ${
            key.split(":").slice(1).join(":")
          } resolves to a non-local symbol.`,
        );
      }
    }
  });

  return issues;
}

function main() {
  const program = ts.createProgram({
    rootNames: CONTRACT_FILES.map((file) => toAbsolutePath(file)),
    options: {
      target: ts.ScriptTarget.ESNext,
      module: ts.ModuleKind.ESNext,
      jsx: ts.JsxEmit.Preserve,
      skipLibCheck: true,
    },
  });
  const checker = program.getTypeChecker();

  const implSource = getProgramSourceFile(program, IMPL_FILE);
  const viewSource = getProgramSourceFile(program, VIEW_FILE);
  const dialogsSource = getProgramSourceFile(program, DIALOGS_FILE);
  const hookSource = getProgramSourceFile(program, HOOK_FILE);

  const hookRequiredKeys = getScopeDestructureKeys(
    hookSource,
    "useGitHistoryPanelInteractions",
    checker,
  );
  const viewRequiredKeys = getScopeDestructureKeys(
    viewSource,
    "renderGitHistoryPanelView",
    checker,
  );
  const dialogsRequiredKeys = getScopeDestructureKeys(
    dialogsSource,
    "renderGitHistoryPanelDialogs",
    checker,
  );

  const hookArgument = getFirstCallArgument(
    implSource,
    "useGitHistoryPanelInteractions",
  );
  const hookProvided = resolveProvidedKeysFromArgument(hookArgument, new Map());

  const viewArgument = getFirstCallArgument(
    implSource,
    "renderGitHistoryPanelView",
  );
  const viewProvided = resolveProvidedKeysFromArgument(viewArgument, new Map());

  const dialogsArgument = getFirstCallArgument(
    viewSource,
    "renderGitHistoryPanelDialogs",
  );
  const dialogsProvided = resolveProvidedKeysFromArgument(
    dialogsArgument,
    new Map([["scope", viewProvided.keys]]),
  );

  const issues = [
    ...checkContract({
      name: "useGitHistoryPanelInteractions",
      requiredKeys: hookRequiredKeys,
      providedKeys: hookProvided.keys,
      unresolvedSpreads: hookProvided.unresolvedSpreads,
    }),
    ...checkContract({
      name: "renderGitHistoryPanelView",
      requiredKeys: viewRequiredKeys,
      providedKeys: viewProvided.keys,
      unresolvedSpreads: viewProvided.unresolvedSpreads,
    }),
    ...checkContract({
      name: "renderGitHistoryPanelDialogs",
      requiredKeys: dialogsRequiredKeys,
      providedKeys: dialogsProvided.keys,
      unresolvedSpreads: dialogsProvided.unresolvedSpreads,
    }),
    ...checkShorthandBindings(
      implSource,
      hookArgument,
      "useGitHistoryPanelInteractions argument",
      checker,
    ),
    ...checkShorthandBindings(
      implSource,
      viewArgument,
      "renderGitHistoryPanelView argument",
      checker,
    ),
    ...checkShorthandBindings(
      viewSource,
      dialogsArgument,
      "renderGitHistoryPanelDialogs argument",
      checker,
    ),
    ...checkRuntimeIdentifierSafety(
      hookSource,
      "useGitHistoryPanelInteractions",
      checker,
    ),
    ...checkRuntimeIdentifierSafety(
      viewSource,
      "renderGitHistoryPanelView",
      checker,
    ),
    ...checkRuntimeIdentifierSafety(
      dialogsSource,
      "renderGitHistoryPanelDialogs",
      checker,
    ),
  ];

  if (issues.length > 0) {
    console.error("check-git-history-runtime-contract: FAILED");
    for (const issue of issues) {
      console.error(`- ${issue}`);
    }
    process.exit(1);
  }

  console.log("check-git-history-runtime-contract: OK");
}

try {
  main();
} catch (error) {
  console.error(
    `check-git-history-runtime-contract: FAILED\n- ${
      error instanceof Error ? error.message : String(error)
    }`,
  );
  process.exit(1);
}
