// -----------------------------------------------------------------------------
// Admin Suite — English (United States) bundle
// -----------------------------------------------------------------------------
// Same key layout as zh-CN.ts.  See that file for the comment on naming
// conventions and translation rules.
// Last reviewed: 2026-07-04.
// -----------------------------------------------------------------------------

export default {
  // ============================================================
  // app
  // ============================================================
  'app.name': 'Admin Suite',
  'app.tagline': 'Desktop admin console',

  // ============================================================
  // common
  // ============================================================
  'common.ok': 'OK',
  'common.cancel': 'Cancel',
  'common.save': 'Save',
  'common.delete': 'Delete',
  'common.edit': 'Edit',
  'common.create': 'Create',
  'common.search': 'Search',
  'common.refresh': 'Refresh',
  'common.confirmDelete': 'Are you sure you want to delete this item?',
  'common.yes': 'Yes',
  'common.no': 'No',
  'common.loading': 'Loading...',
  'common.success': 'Success',
  'common.failed': 'Failed',
  'common.optional': 'optional',
  'common.required': 'required',
  'common.actions': 'Actions',
  'common.code': 'Code',
  'common.name': 'Name',
  'common.status': 'Status',
  'common.description': 'Description',
  'common.sort': 'Sort',
  'common.copy': 'Copy',
  'common.copySuccess': 'Copied',
  'common.copyFailed': 'Copy failed',
  'common.source': 'Source',
  'common.source.builtin': 'built-in',
  'common.source.custom': 'custom',
  'common.builtIn': 'built-in',
  'common.enabled': 'enabled',
  'common.disabled': 'disabled',
  'common.dark': 'Dark',
  'common.light': 'Light',
  'common.language': 'Language',
  'common.theme': 'Theme',
  'common.confirm': 'Confirm',
  'common.back': 'Back',
  'common.empty': 'No data',
  'common.import': 'Import',
  'common.export': 'Export',
  'common.download': 'Download',
  'common.upload': 'Upload',
  'common.submit': 'Submit',
  'common.reset': 'Reset',
  'common.close': 'Close',
  'common.open': 'Open',
  'common.add': 'Add',
  'common.detail': 'Detail',
  'common.placeholder': 'Type here',
  'common.selected': 'Selected',
  'common.available': 'Available',
  'common.total': '{n} items total',

  // ============================================================
  // auth
  // ============================================================
  'auth.login': 'Login',
  'auth.logout': 'Logout',
  'auth.username': 'Username',
  'auth.password': 'Password',
  'auth.passwordRequired': 'Password is required',
  'auth.loginSuccess': 'Login successful',
  'auth.invalidCredentials': 'Invalid username or password',
  'auth.welcomeBack': 'Welcome back',
  'auth.sessionExpired': 'Session expired, please log in again',

  // ============================================================
  // menu
  // ============================================================
  'menu.dashboard': 'Dashboard',
  'menu.system': 'System',
  'menu.users': 'Users',
  'menu.roles': 'Roles',
  'menu.menus': 'Menus',
  'menu.permissions': 'Permissions',
  'menu.themes': 'Themes',
  'menu.locales': 'Languages',
  'menu.audit': 'Audit Log',
  'menu.tools': 'Tools',
  'menu.tools.base': 'Base Converter',
  'menu.tools.json': 'JSON Formatter',
  'menu.tools.datetime': 'Date & Time',
  'menu.tools.sql': 'SQL Formatter',
  'menu.tools.encode': 'URL / HTML',
  'menu.tools.hash': 'Hash Calculator',
  'menu.tools.generate': 'UUID & Password',
  'menu.tools.regex': 'Regex Tester',
  'menu.tools.diff': 'Diff',
  'menu.tools.string': 'String Converter',
  'menu.tools.crypto': 'Crypto',

  // ============================================================
  // notfound
  // ============================================================
  'notfound.title': 'Page not found',
  'notfound.desc': 'The page you visited does not exist or has been removed.',
  'notfound.back': 'Back to dashboard',

  // ============================================================
  // dashboard
  // ============================================================
  'dashboard.title': 'Dashboard',
  'dashboard.welcome': 'Welcome to Admin Suite',
  'dashboard.desc': 'Use the sidebar to manage users, roles, menus, themes and languages.',
  'dashboard.info': 'App Info',
  'dashboard.dataDir': 'Data directory',
  'dashboard.dbPath': 'Database path',
  'dashboard.migrationsDir': 'Migrations directory',
  'dashboard.defaultAdmin': 'Default admin',

  // ============================================================
  // users
  // ============================================================
  'users.title': 'User Management',
  'users.create': 'New user',
  'users.columns.username': 'Username',
  'users.columns.displayName': 'Display name',
  'users.columns.email': 'Email',
  'users.columns.roles': 'Roles',
  'users.columns.status': 'Status',
  'users.columns.lastLogin': 'Last login',
  'users.password': 'Password',
  'users.passwordHelp': 'Leave blank to keep current',
  'users.statusActive': 'Active',
  'users.statusDisabled': 'Disabled',
  'users.validation.usernameRequired': 'Username is required',
  'users.validation.displayNameRequired': 'Display name is required',
  'users.validation.passwordTooShort': 'Password must be at least 6 characters',
  'users.validation.emailInvalid': 'Invalid email format',
  'users.delete.confirm': 'Delete user {name}?',

  // ============================================================
  // roles
  // ============================================================
  'roles.title': 'Role Management',
  'roles.create': 'New role',
  'roles.columns.code': 'Code',
  'roles.columns.name': 'Name',
  'roles.columns.permissions': 'Permissions',
  'roles.columns.builtIn': 'Built-in',
  'roles.columns.users': 'Users',
  'roles.assignMenus': 'Assign menus',
  'roles.assignPermissions': 'Assign permissions',
  'roles.transfer.available': 'Available',
  'roles.transfer.selected': 'Selected',
  'roles.validation.codeRequired': 'Code is required',
  'roles.validation.nameRequired': 'Name is required',
  'roles.delete.confirm': 'Delete role {name}?',

  // ============================================================
  // perms
  // ============================================================
  'perms.title': 'Permissions',
  'perms.columns.code': 'Permission code',
  'perms.columns.resource': 'Resource',
  'perms.columns.action': 'Action',
  'perms.columns.name': 'Name',
  'perms.columns.description': 'Description',
  'perms.help': 'Permissions are seeded by Flyway migrations on the backend; they cannot be created or removed from the UI.',

  // ============================================================
  // menus
  // ============================================================
  'menus.title': 'Menu Management',
  'menus.create': 'New menu',
  'menus.createChild': 'New child menu',
  'menus.titleKey': 'Title (i18n key)',
  'menus.titleKeyHelp': 'Optional. Looked up via t() so locale switching works.',
  'menus.columns.path': 'Path',
  'menus.columns.icon': 'Icon',
  'menus.columns.permission': 'Permission',
  'menus.columns.visible': 'Visible',
  'menus.columns.parent': 'Parent',
  'menus.columns.type': 'Type',
  'menus.iconPlaceholder': 'user-filled',
  'menus.permissionPlaceholder': 'user:read',
  'menus.titleKeyPlaceholder': 'menu.users',
  'menus.delete.confirm': 'Delete menu {name}? Its children will be deleted as well.',
  'menus.rootMenu': 'Top-level',

  // ============================================================
  // themes
  // ============================================================
  'themes.title': 'Theme Management',
  'themes.import': 'Import theme',
  'themes.active': 'Active',
  'themes.activate': 'Activate',
  'themes.importHelp':
    'Drop a JSON file with the shape { "id": "...", "label": "...", "isDark": false, "tokens": { "--color-...": "#hex" } }.',
  'themes.importSuccess': 'Theme imported',
  'themes.importFailed': 'Theme import failed',
  'themes.delete.confirm': 'Delete theme {name}?',
  'themes.preview.dark': 'dark',
  'themes.preview.light': 'light',

  // ============================================================
  // locales
  // ============================================================
  'locales.title': 'Language Management',
  'locales.import': 'Import language pack',
  'locales.export': 'Export',
  'locales.exportOne': 'Export this language',
  'locales.exportDialog': 'Export language pack',
  'locales.exportSource': 'Source locale',
  'locales.exportTargetCode': 'Target code',
  'locales.exportTargetLabel': 'Target label',
  'locales.exportTargetLabelPlaceholder': 'e.g. 日本語',
  'locales.exportFillEmpty': 'Fill missing keys',
  'locales.exportFillEmptyHelp': 'When on, the export includes every key the app knows about, with empty strings for translations you still need to fill in.',
  'locales.download': 'Download',
  'locales.preview': 'Preview',
  'locales.code': 'Code',
  'locales.name': 'Name',
  'locales.messages': 'Messages',
  'locales.noLocale': 'No locale available to export',
  'locales.importHelp':
    'Drop a JSON file with the shape { "id": "xx-YY", "label": "Name", "messages": { "common.ok": "OK", ... } }.',
  'locales.importSuccess': 'Language pack imported',
  'locales.importFailed': 'Language pack import failed',
  'locales.activateSuccess': 'Language switched',

  // ============================================================
  // audit
  // ============================================================
  'audit.title': 'Audit Log',
  'audit.columns.action': 'Action',
  'audit.columns.actor': 'Actor',
  'audit.columns.target': 'Target',
  'audit.columns.resource': 'Resource',
  'audit.columns.payload': 'Payload',
  'audit.columns.time': 'Time',
  'audit.actionFilter': 'Filter by action',
  'audit.actorFilter': 'Filter by actor',
  'audit.empty': 'No audit records',

  // ============================================================
  // tools.base
  // ============================================================
  'tools.base.title': 'Base Converter',
  'tools.base.modeNumber': 'Number',
  'tools.base.modeText': 'Text / Bytes',
  'tools.base.input': 'Input',
  'tools.base.inputPlaceholder': 'e.g. 255 (decimal), ff (hex), 11111111 (binary)',
  'tools.base.fromBase': 'From base',
  'tools.base.fromBase.binary': 'Binary (2)',
  'tools.base.fromBase.octal': 'Octal (8)',
  'tools.base.fromBase.decimal': 'Decimal (10)',
  'tools.base.fromBase.hex': 'Hex (16)',
  'tools.base.textInput': 'Text input',
  'tools.base.textPlaceholder': 'Type or paste text to convert',
  'tools.base.invalid': 'invalid',
  'tools.base.output.bin': 'BIN (2)',
  'tools.base.output.oct': 'OCT (8)',
  'tools.base.output.dec': 'DEC (10)',
  'tools.base.output.hex': 'HEX (16)',
  'tools.base.output.hexBytes': 'HEX bytes',
  'tools.base.output.binBytes': 'BIN bytes',
  'tools.base.output.base64': 'Base64',
  'tools.base.output.url': 'URL-encoded',
  'tools.base.output.charCodes': 'Char codes',
  'tools.base.output.length': 'Length (B)',

  // ============================================================
  // tools.json
  // ============================================================
  'tools.json.title': 'JSON Formatter',
  'tools.json.input': 'Input',
  'tools.json.placeholder': 'Paste JSON here, or load a file...',
  'tools.json.format': 'Format',
  'tools.json.minify': 'Minify',
  'tools.json.indent': 'Indent',
  'tools.json.sortKeys': 'Sort keys',
  'tools.json.tree': 'Tree',
  'tools.json.viewTree': 'Tree',
  'tools.json.viewText': 'Pretty',
  'tools.json.empty': 'Paste JSON on the left to see the tree.',
  'tools.json.loadFile': 'Load file',
  'tools.json.parseError': 'Invalid JSON',
  'tools.json.status': '{lines} lines, valid JSON',
  'tools.json.indent.tabs': 'Tab',
  'tools.json.indent.2': '2 spaces',
  'tools.json.indent.4': '4 spaces',

  // ============================================================
  // tools.datetime
  // ============================================================
  'tools.datetime.title': 'Date & Time',
  'tools.datetime.now': 'Now',
  'tools.datetime.unix': 'Unix timestamp',
  'tools.datetime.iso': 'ISO / Timezone',
  'tools.datetime.timestamp': 'Timestamp',
  'tools.datetime.unit': 'Unit',
  'tools.datetime.seconds': 'Seconds',
  'tools.datetime.millis': 'Milliseconds',
  'tools.datetime.isoInput': 'ISO 8601',
  'tools.datetime.isoPlaceholder': '2026-07-02T00:00:00Z',
  'tools.datetime.tz': 'Timezone',
  'tools.datetime.custom': 'Format',
  'tools.datetime.customPlaceholder': 'YYYY-MM-DD HH:mm:ss',
  'tools.datetime.outputs': 'Outputs',
  'tools.datetime.iso8601': 'ISO 8601 (UTC)',
  'tools.datetime.utc': 'UTC',
  'tools.datetime.local': 'Local',
  'tools.datetime.formatted': 'Custom format',
  'tools.datetime.dayOfWeek': 'Day of week',
  'tools.datetime.weekOfYear': 'ISO week',
  'tools.datetime.dayOfYear': 'Day of year',
  'tools.datetime.daysInYear': 'Days in year',
  'tools.datetime.leapYear': 'Leap year',
  'tools.datetime.epochDiff': 'Days since epoch',
  'tools.datetime.offset': 'Shift by duration',
  'tools.datetime.op': 'Operation',
  'tools.datetime.plus': '+ Add',
  'tools.datetime.minus': '− Subtract',
  'tools.datetime.amount': 'Amount',
  'tools.datetime.result': 'Result',

  // ============================================================
  // tools.hash
  // ============================================================
  'tools.hash.title': 'Hash Calculator',
  'tools.hash.fromText': 'Text',
  'tools.hash.fromFile': 'File',
  'tools.hash.textPlaceholder': 'Type or paste text to hash...',
  'tools.hash.dropOrClick': 'Drop file here, or click to select',
  'tools.hash.algorithm': 'Algorithm',
  'tools.hash.digest': 'Digest',
  'tools.hash.hmac': 'HMAC',
  'tools.hash.hmacKey': 'HMAC key',
  'tools.hash.hmacKeyPlaceholder': 'shared secret',
  'tools.hash.algo.MD5': 'MD5',
  'tools.hash.algo.SHA-1': 'SHA-1',
  'tools.hash.algo.SHA-256': 'SHA-256',
  'tools.hash.algo.SHA-384': 'SHA-384',
  'tools.hash.algo.SHA-512': 'SHA-512',

  // ============================================================
  // tools.gen
  // ============================================================
  'tools.gen.title': 'UUID & Password',
  'tools.gen.tab.uuid': 'UUID',
  'tools.gen.tab.password': 'Password',
  'tools.gen.tab.passphrase': 'Passphrase',
  'tools.gen.password': 'Password',
  'tools.gen.passphrase': 'Passphrase',
  'tools.gen.uuid': 'UUID',
  'tools.gen.version': 'Version',
  'tools.gen.version.v4': 'v4 (random)',
  'tools.gen.version.v7': 'v7 (time-sortable)',
  'tools.gen.version.nil': 'nil UUID',
  'tools.gen.uppercase': 'Uppercase',
  'tools.gen.lowercase': 'Lowercase',
  'tools.gen.digits': 'Digits',
  'tools.gen.symbols': 'Symbols',
  'tools.gen.exclude': 'Exclude chars',
  'tools.gen.excludePlaceholder': '0OIl',
  'tools.gen.hyphens': 'Keep hyphens',
  'tools.gen.braces': 'Wrap in braces',
  'tools.gen.length': 'Length',
  'tools.gen.count': 'Count',
  'tools.gen.generate': 'Generate',
  'tools.gen.output': 'Output',
  'tools.gen.clickGenerate': 'Click "Generate" to create items',
  'tools.gen.enableOne': 'Enable at least one character set',
  'tools.gen.strength': 'Strength',
  'tools.gen.weak': 'Weak',
  'tools.gen.medium': 'Medium',
  'tools.gen.strong': 'Strong',
  'tools.gen.wordCount': 'Word count',
  'tools.gen.separator': 'Separator',
  'tools.gen.capitalize': 'Capitalize',
  'tools.gen.appendDigits': 'Append digits',

  // ============================================================
  // tools.encode
  // ============================================================
  'tools.encode.title': 'URL / HTML Encode',
  'tools.encode.input': 'Input',
  'tools.encode.output': 'Output',
  'tools.encode.placeholder': 'Type or paste text...',
  'tools.encode.swap': 'Swap',
  'tools.encode.loadFile': 'Load file',
  'tools.encode.mode.url': 'URL encode',
  'tools.encode.mode.url-dec': 'URL decode',
  'tools.encode.mode.html': 'HTML entity encode',
  'tools.encode.mode.html-dec': 'HTML entity decode',
  'tools.encode.mode.b64': 'Base64 encode',
  'tools.encode.mode.b64-dec': 'Base64 decode',
  'tools.encode.mode.hex': 'Hex encode (bytes)',
  'tools.encode.mode.hex-dec': 'Hex decode (bytes)',
  'tools.encode.mode.unicode-esc': 'Unicode escape',
  'tools.encode.mode.unicode-unesc': 'Unicode unescape',

  // ============================================================
  // tools.regex
  // ============================================================
  'tools.regex.title': 'Regex Tester',
  'tools.regex.pattern': 'Pattern',
  'tools.regex.flags': 'Flags',
  'tools.regex.input': 'Test string',
  'tools.regex.matches': 'Matches',
  'tools.regex.noMatch': 'No matches',
  'tools.regex.match': 'Match',
  'tools.regex.index': 'Index',
  'tools.regex.groups': 'Groups',
  'tools.regex.replace': 'Replacement',
  'tools.regex.replacePlaceholder': 'Use $1, $2 for backrefs',
  'tools.regex.preview': 'Live preview',

  // ============================================================
  // tools.sql
  // ============================================================
  'tools.sql.title': 'SQL Formatter',
  'tools.sql.input': 'Input',
  'tools.sql.output': 'Output',
  'tools.sql.placeholder': 'Paste SQL here...',
  'tools.sql.format': 'Format',
  'tools.sql.minify': 'Minify',
  'tools.sql.uppercase': 'Uppercase keywords',
  'tools.sql.parseError': 'Invalid SQL',
  'tools.sql.language': 'Dialect',

  // ============================================================
  // tools.diff
  // ============================================================
  'tools.diff.title': 'Diff',
  'tools.diff.original': 'Original',
  'tools.diff.modified': 'Modified',
  'tools.diff.result': 'Result',
  'tools.diff.split': 'Split',
  'tools.diff.unified': 'Unified',
  'tools.diff.line': 'Line',
  'tools.diff.word': 'Word',
  'tools.diff.char': 'Char',
  'tools.diff.chars': 'chars',
  'tools.diff.unchanged': 'unchanged',
  'tools.diff.labelA': 'A',
  'tools.diff.labelB': 'B',
  'tools.diff.clear': 'Clear',

  // ============================================================
  // tools.string
  // ============================================================
  'tools.string.title': 'String Converter',
  'tools.string.swap': 'Swap',
  'tools.string.input': 'Input',
  'tools.string.output': 'Output',
  'tools.string.placeholder': 'Type or paste text to convert...',
  'tools.string.format': 'Format',
  'tools.string.mode': 'Mode',
  'tools.string.direction': 'Direction',
  'tools.string.encode': 'Encode',
  'tools.string.decode': 'Decode',
  'tools.string.tabUnicode': 'Unicode',
  'tools.string.tabHtml': 'HTML',
  'tools.string.tabUrl': 'URL',
  'tools.string.tabNormalize': 'Normalize',
  'tools.string.tabAscii': 'ASCII',
  'tools.string.tabString': 'String literal',
  'tools.string.unicodeEscape': '\\uXXXX escape',
  'tools.string.unicodeHex': '\\xXX escape',
  'tools.string.unicodeCode': 'U+XXXX notation',
  'tools.string.unicodeDecimal': '&#NNNN; numeric',
  'tools.string.unicodeRaw': 'Raw (passthrough)',
  'tools.string.htmlNumeric': 'Numeric (&#NNNN;)',
  'tools.string.htmlHex': 'Hex (&#xNNNN;)',
  'tools.string.htmlNamed': 'Named (&amp;name;)',
  'tools.string.htmlAll': 'All (named for common, numeric otherwise)',
  'tools.string.form': 'Form',
  'tools.string.normalizeHelp': 'Normalize a string to one of the four Unicode normalization forms.',
  'tools.string.asciiStrip': 'Strip non-ASCII',
  'tools.string.asciiReplace': 'Replace non-ASCII with ?',
  'tools.string.asciiEscape': 'Escape as \\uXXXX',
  'tools.string.asciiCodepoints': 'Show code points',
  'tools.string.sJson': 'JSON string',
  'tools.string.sJs': 'JavaScript string',
  'tools.string.sCss': 'CSS string',
  'tools.string.urlDecodeError': 'Could not decode URL',

  // ============================================================
  // tools.crypto
  // ============================================================
  'tools.crypto.title': 'Crypto',
  'tools.crypto.aesGcm': 'AES-GCM',
  'tools.crypto.aesCbc': 'AES-CBC',
  'tools.crypto.rsa': 'RSA-OAEP',
  'tools.crypto.rc4': 'RC4',
  'tools.crypto.caesar': 'Caesar',
  'tools.crypto.vigenere': 'Vigenère',
  'tools.crypto.xor': 'XOR',
  'tools.crypto.key': 'Key',
  'tools.crypto.iv': 'IV / Nonce',
  'tools.crypto.aad': 'AAD',
  'tools.crypto.shift': 'Shift',
  'tools.crypto.plaintext': 'Plaintext',
  'tools.crypto.ciphertext': 'Ciphertext',
  'tools.crypto.encrypt': 'Encrypt',
  'tools.crypto.decrypt': 'Decrypt',
  'tools.crypto.encryptWithPublic': 'Encrypt with public',
  'tools.crypto.decryptWithPrivate': 'Decrypt with private',
  'tools.crypto.generateKeys': 'Generate key pair',
  'tools.crypto.publicKey': 'Public key',
  'tools.crypto.privateKey': 'Private key',
  'tools.crypto.aesKeyPlaceholder': 'Base64-encoded 16/24/32-byte key',
  'tools.crypto.aesGcmIvPlaceholder': 'Base64-encoded 12-byte nonce',
  'tools.crypto.aesCbcIvPlaceholder': 'Base64-encoded 16-byte IV',
  'tools.crypto.aadPlaceholder': 'Optional Additional Authenticated Data',
  'tools.crypto.randomKey': 'Random key',
  'tools.crypto.randomIv': 'Random IV',
  'tools.crypto.aesKeyLengthError': 'AES key must be 16, 24, or 32 bytes (128/192/256 bits).',
  'tools.crypto.gcmIvLengthError': 'GCM nonce must be 12 bytes.',
  'tools.crypto.cbcIvLengthError': 'CBC IV must be 16 bytes.',
  'tools.crypto.rsaKeySize': 'Key size',
  'tools.crypto.rsaKeyGenerated': 'Key pair generated — encrypt with the public key, decrypt with the private key.',
  'tools.crypto.rsaKeyMissing': 'Generate a key pair first.',
  'tools.crypto.rsaHelp': 'RSA-OAEP with SHA-256. Generates a fresh key pair in your browser; nothing is sent to any server.',
  'tools.crypto.vigenereKeyPlaceholder': 'A–Z letters (other chars are stripped)',
  'tools.crypto.vigenereKeyEmpty': 'Vigenère key must contain A–Z letters.',
  'tools.crypto.xorKeyPlaceholder': 'Bytes as hex (e.g. deadbeef) or plain text',
  'tools.crypto.xorKeyHelp': 'Repeats over the input.  Symmetric: encrypt = decrypt.',
  'tools.crypto.xorKeyEmpty': 'XOR key cannot be empty.',
  'tools.crypto.rc4Note': 'RC4 is a legacy stream cipher — not secure for new code, kept here for interoperability.'
}