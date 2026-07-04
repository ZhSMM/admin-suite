// -----------------------------------------------------------------------------
// Admin Suite — Chinese (Simplified) bundle
// -----------------------------------------------------------------------------
// Convention:
//   - All user-visible strings live here.  No hard-coded text in .vue files.
//   - Key naming: `<scope>.<page-or-section>.<element>`
//     scopes:
//       app      — application-level (title, tagline, version, etc.)
//       common   — shared across multiple pages (buttons, statuses, words)
//       auth     — login / logout / account
//       menu     — sidebar entries
//       notfound — 404 page
//       dashboard — dashboard page
//       users    — user management page
//       roles    — role management page
//       perms    — permission list page
//       menus    — menu management page
//       themes   — theme management page
//       locales  — language management page
//       audit    — audit log page
//       tools.<name> — each individual tool
//   - Re-use `common.*` whenever the same English/Chinese word appears in 2+
//     different pages; never duplicate it under a page-specific key.
//   - Translators: keep keys present in BOTH zh-CN.ts AND en-US.ts.  The "fill
//     missing keys" export dialog uses the union of all known keys.
//
// Last reviewed: 2026-07-04.
// -----------------------------------------------------------------------------

export default {
  // ============================================================
  // app
  // ============================================================
  'app.name': 'Admin Suite',
  'app.tagline': '桌面端后台管理系统',

  // ============================================================
  // common — shared across multiple pages
  // ============================================================
  'common.ok': '确定',
  'common.cancel': '取消',
  'common.save': '保存',
  'common.delete': '删除',
  'common.edit': '编辑',
  'common.create': '新建',
  'common.search': '搜索',
  'common.refresh': '刷新',
  'common.confirmDelete': '确定要删除这条记录吗?',
  'common.yes': '是',
  'common.no': '否',
  'common.loading': '加载中...',
  'common.success': '操作成功',
  'common.failed': '操作失败',
  'common.optional': '可选',
  'common.required': '必填',
  'common.actions': '操作',
  'common.code': '编码',
  'common.name': '名称',
  'common.status': '状态',
  'common.description': '描述',
  'common.sort': '排序',
  'common.copy': '复制',
  'common.copySuccess': '已复制',
  'common.copyFailed': '复制失败',
  'common.source': '来源',
  'common.source.builtin': '内置',
  'common.source.custom': '自定义',
  'common.builtIn': '内置',
  'common.enabled': '启用',
  'common.disabled': '禁用',
  'common.dark': '深色',
  'common.light': '浅色',
  'common.language': '语言',
  'common.theme': '主题',
  'common.confirm': '确认',
  'common.back': '返回',
  'common.empty': '暂无数据',
  'common.import': '导入',
  'common.export': '导出',
  'common.download': '下载',
  'common.upload': '上传',
  'common.submit': '提交',
  'common.reset': '重置',
  'common.close': '关闭',
  'common.open': '打开',
  'common.add': '新增',
  'common.detail': '详情',
  'common.placeholder': '请输入',
  'common.selected': '已选',
  'common.available': '可选',
  'common.total': '共 {n} 条',

  // ============================================================
  // auth — login & session
  // ============================================================
  'auth.login': '登录',
  'auth.logout': '退出登录',
  'auth.username': '用户名',
  'auth.password': '密码',
  'auth.passwordRequired': '请输入密码',
  'auth.loginSuccess': '登录成功',
  'auth.invalidCredentials': '用户名或密码错误',
  'auth.welcomeBack': '欢迎回来',
  'auth.sessionExpired': '会话已过期,请重新登录',

  // ============================================================
  // menu — sidebar entries (title keys for runtime-built menus)
  // ============================================================
  'menu.dashboard': '仪表盘',
  'menu.system': '系统管理',
  'menu.users': '用户管理',
  'menu.roles': '角色管理',
  'menu.menus': '菜单管理',
  'menu.permissions': '权限管理',
  'menu.themes': '主题管理',
  'menu.locales': '语言管理',
  'menu.audit': '审计日志',
  'menu.tools': '工具',
  'menu.tools.base': '进制转换',
  'menu.tools.json': 'JSON 格式化',
  'menu.tools.datetime': '时间日期',
  'menu.tools.sql': 'SQL 格式化',
  'menu.tools.encode': 'URL / HTML',
  'menu.tools.hash': '哈希计算',
  'menu.tools.generate': 'UUID 与密码',
  'menu.tools.regex': '正则测试',
  'menu.tools.diff': '文本对比',
  'menu.tools.string': '字符串转换',
  'menu.tools.crypto': '加解密',

  // ============================================================
  // notfound
  // ============================================================
  'notfound.title': '页面不存在',
  'notfound.desc': '您访问的页面不存在或已被移除。',
  'notfound.back': '返回仪表盘',

  // ============================================================
  // dashboard
  // ============================================================
  'dashboard.title': '仪表盘',
  'dashboard.welcome': '欢迎使用 Admin Suite',
  'dashboard.desc': '使用侧边栏管理用户、角色、菜单、主题和语言。',
  'dashboard.info': '应用信息',
  'dashboard.dataDir': '数据目录',
  'dashboard.dbPath': '数据库路径',
  'dashboard.migrationsDir': '迁移目录',
  'dashboard.defaultAdmin': '默认管理员',

  // ============================================================
  // users
  // ============================================================
  'users.title': '用户管理',
  'users.create': '新建用户',
  'users.columns.username': '用户名',
  'users.columns.displayName': '显示名',
  'users.columns.email': '邮箱',
  'users.columns.roles': '角色',
  'users.columns.status': '状态',
  'users.columns.lastLogin': '最后登录',
  'users.password': '密码',
  'users.passwordHelp': '留空则不修改',
  'users.statusActive': '启用',
  'users.statusDisabled': '禁用',
  'users.validation.usernameRequired': '请输入用户名',
  'users.validation.displayNameRequired': '请输入显示名',
  'users.validation.passwordTooShort': '密码至少 6 位',
  'users.validation.emailInvalid': '邮箱格式不正确',
  'users.delete.confirm': '确定要删除用户 {name} 吗?',

  // ============================================================
  // roles
  // ============================================================
  'roles.title': '角色管理',
  'roles.create': '新建角色',
  'roles.columns.code': '编码',
  'roles.columns.name': '名称',
  'roles.columns.permissions': '权限',
  'roles.columns.builtIn': '内置',
  'roles.columns.users': '关联用户',
  'roles.assignMenus': '分配菜单',
  'roles.assignPermissions': '分配权限',
  'roles.transfer.available': '可选权限',
  'roles.transfer.selected': '已分配权限',
  'roles.validation.codeRequired': '请输入编码',
  'roles.validation.nameRequired': '请输入名称',
  'roles.delete.confirm': '确定要删除角色 {name} 吗?',

  // ============================================================
  // perms
  // ============================================================
  'perms.title': '权限管理',
  'perms.columns.code': '权限编码',
  'perms.columns.resource': '资源',
  'perms.columns.action': '操作',
  'perms.columns.name': '名称',
  'perms.columns.description': '描述',
  'perms.help': '权限由后端在 Flyway 迁移中预置,前端不可新增或删除。',

  // ============================================================
  // menus
  // ============================================================
  'menus.title': '菜单管理',
  'menus.create': '新建菜单',
  'menus.createChild': '新建子菜单',
  'menus.titleKey': '标题 (i18n key)',
  'menus.titleKeyHelp': '可选。填了之后通过 t() 查找,支持语言切换。',
  'menus.columns.path': '路径',
  'menus.columns.icon': '图标',
  'menus.columns.permission': '权限',
  'menus.columns.visible': '可见',
  'menus.columns.parent': '父菜单',
  'menus.columns.type': '类型',
  'menus.iconPlaceholder': 'user-filled',
  'menus.permissionPlaceholder': 'user:read',
  'menus.titleKeyPlaceholder': 'menu.users',
  'menus.delete.confirm': '确定要删除菜单 {name} 吗? 子菜单也会一并删除。',
  'menus.rootMenu': '顶级菜单',

  // ============================================================
  // themes
  // ============================================================
  'themes.title': '主题管理',
  'themes.import': '导入主题',
  'themes.active': '当前',
  'themes.activate': '启用',
  'themes.importHelp':
    '上传 JSON 文件, 格式: { "id": "...", "label": "...", "isDark": false, "tokens": { "--color-...": "#hex" } }。',
  'themes.importSuccess': '主题导入成功',
  'themes.importFailed': '主题导入失败',
  'themes.delete.confirm': '确定要删除主题 {name} 吗?',
  'themes.preview.dark': '深色',
  'themes.preview.light': '浅色',

  // ============================================================
  // locales
  // ============================================================
  'locales.title': '语言管理',
  'locales.import': '导入语言包',
  'locales.export': '导出',
  'locales.exportOne': '导出此语言',
  'locales.exportDialog': '导出语言包',
  'locales.exportSource': '源语言',
  'locales.exportTargetCode': '目标编码',
  'locales.exportTargetLabel': '目标名称',
  'locales.exportTargetLabelPlaceholder': '如: 日本語',
  'locales.exportFillEmpty': '补齐缺失的 key',
  'locales.exportFillEmptyHelp': '开启后,导出文件会包含所有应用用到的 key,还没翻译的用空字符串占位。',
  'locales.download': '下载',
  'locales.preview': '预览',
  'locales.code': '编码',
  'locales.name': '名称',
  'locales.messages': '消息',
  'locales.noLocale': '没有可导出的语言',
  'locales.importHelp':
    '上传 JSON 文件, 格式: { "id": "xx-YY", "label": "名称", "messages": { "common.ok": "OK", ... } }。',
  'locales.importSuccess': '语言包导入成功',
  'locales.importFailed': '语言包导入失败',
  'locales.activateSuccess': '语言切换成功',

  // ============================================================
  // audit
  // ============================================================
  'audit.title': '审计日志',
  'audit.columns.action': '动作',
  'audit.columns.actor': '操作人',
  'audit.columns.target': '对象',
  'audit.columns.resource': '资源',
  'audit.columns.payload': '载荷',
  'audit.columns.time': '时间',
  'audit.actionFilter': '按动作过滤',
  'audit.actorFilter': '按操作人过滤',
  'audit.empty': '暂无审计记录',

  // ============================================================
  // tools.base
  // ============================================================
  'tools.base.title': '进制转换',
  'tools.base.modeNumber': '数字',
  'tools.base.modeText': '文本 / 字节',
  'tools.base.input': '输入',
  'tools.base.inputPlaceholder': '例: 255(十进制) / ff(十六进制) / 11111111(二进制)',
  'tools.base.fromBase': '输入进制',
  'tools.base.fromBase.binary': '二进制 (2)',
  'tools.base.fromBase.octal': '八进制 (8)',
  'tools.base.fromBase.decimal': '十进制 (10)',
  'tools.base.fromBase.hex': '十六进制 (16)',
  'tools.base.textInput': '文本输入',
  'tools.base.textPlaceholder': '输入或粘贴文本进行转换',
  'tools.base.invalid': '无效',
  'tools.base.output.bin': 'BIN (2)',
  'tools.base.output.oct': 'OCT (8)',
  'tools.base.output.dec': 'DEC (10)',
  'tools.base.output.hex': 'HEX (16)',
  'tools.base.output.hexBytes': 'HEX 字节',
  'tools.base.output.binBytes': 'BIN 字节',
  'tools.base.output.base64': 'Base64',
  'tools.base.output.url': 'URL-encoded',
  'tools.base.output.charCodes': '字符码点',
  'tools.base.output.length': '字节长度',

  // ============================================================
  // tools.json
  // ============================================================
  'tools.json.title': 'JSON 格式化',
  'tools.json.input': '输入',
  'tools.json.placeholder': '在此粘贴 JSON,或加载文件...',
  'tools.json.format': '格式化',
  'tools.json.minify': '压缩',
  'tools.json.indent': '缩进',
  'tools.json.sortKeys': '键名排序',
  'tools.json.tree': '树视图',
  'tools.json.viewTree': '树视图',
  'tools.json.viewText': '美化文本',
  'tools.json.empty': '左侧粘贴 JSON 即可查看树结构。',
  'tools.json.loadFile': '加载文件',
  'tools.json.parseError': 'JSON 解析错误',
  'tools.json.status': '{lines} 行,JSON 有效',
  'tools.json.indent.tabs': 'Tab',
  'tools.json.indent.2': '2 空格',
  'tools.json.indent.4': '4 空格',

  // ============================================================
  // tools.datetime
  // ============================================================
  'tools.datetime.title': '时间日期',
  'tools.datetime.now': '现在',
  'tools.datetime.unix': 'Unix 时间戳',
  'tools.datetime.iso': 'ISO / 时区',
  'tools.datetime.timestamp': '时间戳',
  'tools.datetime.unit': '单位',
  'tools.datetime.seconds': '秒',
  'tools.datetime.millis': '毫秒',
  'tools.datetime.isoInput': 'ISO 8601',
  'tools.datetime.isoPlaceholder': '2026-07-02T00:00:00Z',
  'tools.datetime.tz': '时区',
  'tools.datetime.custom': '格式',
  'tools.datetime.customPlaceholder': 'YYYY-MM-DD HH:mm:ss',
  'tools.datetime.outputs': '输出',
  'tools.datetime.iso8601': 'ISO 8601 (UTC)',
  'tools.datetime.utc': 'UTC',
  'tools.datetime.local': '本地',
  'tools.datetime.formatted': '自定义格式',
  'tools.datetime.dayOfWeek': '星期',
  'tools.datetime.weekOfYear': 'ISO 周',
  'tools.datetime.dayOfYear': '年内第几天',
  'tools.datetime.daysInYear': '全年天数',
  'tools.datetime.leapYear': '闰年',
  'tools.datetime.epochDiff': '距 1970-01-01 天数',
  'tools.datetime.offset': '时间偏移',
  'tools.datetime.op': '操作',
  'tools.datetime.plus': '+ 增加',
  'tools.datetime.minus': '− 减少',
  'tools.datetime.amount': '数量',
  'tools.datetime.result': '结果',

  // ============================================================
  // tools.hash
  // ============================================================
  'tools.hash.title': '哈希计算',
  'tools.hash.fromText': '文本',
  'tools.hash.fromFile': '文件',
  'tools.hash.textPlaceholder': '输入或粘贴要哈希的文本...',
  'tools.hash.dropOrClick': '拖拽文件到此,或点击选择',
  'tools.hash.algorithm': '算法',
  'tools.hash.digest': '摘要',
  'tools.hash.hmac': 'HMAC',
  'tools.hash.hmacKey': 'HMAC 密钥',
  'tools.hash.hmacKeyPlaceholder': '共享密钥',
  'tools.hash.algo.MD5': 'MD5',
  'tools.hash.algo.SHA-1': 'SHA-1',
  'tools.hash.algo.SHA-256': 'SHA-256',
  'tools.hash.algo.SHA-384': 'SHA-384',
  'tools.hash.algo.SHA-512': 'SHA-512',

  // ============================================================
  // tools.gen (UUID & Password & Passphrase)
  // ============================================================
  'tools.gen.title': 'UUID 与密码',
  'tools.gen.tab.uuid': 'UUID',
  'tools.gen.tab.password': '密码',
  'tools.gen.tab.passphrase': '口令短语',
  'tools.gen.password': '密码',
  'tools.gen.passphrase': '口令短语',
  'tools.gen.uuid': 'UUID',
  'tools.gen.version': '版本',
  'tools.gen.version.v4': 'v4 (随机)',
  'tools.gen.version.v7': 'v7 (按时间排序)',
  'tools.gen.version.nil': 'nil UUID',
  'tools.gen.uppercase': '大写字母',
  'tools.gen.lowercase': '小写字母',
  'tools.gen.digits': '数字',
  'tools.gen.symbols': '符号',
  'tools.gen.exclude': '排除字符',
  'tools.gen.excludePlaceholder': '0OIl',
  'tools.gen.hyphens': '保留连字符',
  'tools.gen.braces': '花括号包裹',
  'tools.gen.length': '长度',
  'tools.gen.count': '数量',
  'tools.gen.generate': '生成',
  'tools.gen.output': '输出',
  'tools.gen.clickGenerate': '点击"生成"创建条目',
  'tools.gen.enableOne': '至少启用一种字符集',
  'tools.gen.strength': '强度',
  'tools.gen.weak': '弱',
  'tools.gen.medium': '中',
  'tools.gen.strong': '强',
  'tools.gen.wordCount': '单词数',
  'tools.gen.separator': '分隔符',
  'tools.gen.capitalize': '首字母大写',
  'tools.gen.appendDigits': '追加数字',

  // ============================================================
  // tools.encode (URL / HTML / Base64 / Hex / Unicode)
  // ============================================================
  'tools.encode.title': 'URL / HTML 编码',
  'tools.encode.input': '输入',
  'tools.encode.output': '输出',
  'tools.encode.placeholder': '输入或粘贴文本...',
  'tools.encode.swap': '交换',
  'tools.encode.loadFile': '加载文件',
  'tools.encode.mode.url': 'URL 编码',
  'tools.encode.mode.url-dec': 'URL 解码',
  'tools.encode.mode.html': 'HTML 实体编码',
  'tools.encode.mode.html-dec': 'HTML 实体解码',
  'tools.encode.mode.b64': 'Base64 编码',
  'tools.encode.mode.b64-dec': 'Base64 解码',
  'tools.encode.mode.hex': 'Hex 编码(字节)',
  'tools.encode.mode.hex-dec': 'Hex 解码(字节)',
  'tools.encode.mode.unicode-esc': 'Unicode 转义',
  'tools.encode.mode.unicode-unesc': 'Unicode 还原',

  // ============================================================
  // tools.regex
  // ============================================================
  'tools.regex.title': '正则测试',
  'tools.regex.pattern': '模式',
  'tools.regex.flags': '标志',
  'tools.regex.input': '测试字符串',
  'tools.regex.matches': '匹配',
  'tools.regex.noMatch': '无匹配',
  'tools.regex.match': '匹配',
  'tools.regex.index': '位置',
  'tools.regex.groups': '分组',
  'tools.regex.replace': '替换',
  'tools.regex.replacePlaceholder': '用 $1、$2 引用分组',
  'tools.regex.preview': '实时预览',

  // ============================================================
  // tools.sql
  // ============================================================
  'tools.sql.title': 'SQL 格式化',
  'tools.sql.input': '输入',
  'tools.sql.output': '输出',
  'tools.sql.placeholder': '在此粘贴 SQL...',
  'tools.sql.format': '格式化',
  'tools.sql.minify': '压缩',
  'tools.sql.uppercase': '关键字大写',
  'tools.sql.parseError': 'SQL 解析错误',
  'tools.sql.language': '方言',

  // ============================================================
  // tools.diff
  // ============================================================
  'tools.diff.title': '文本对比',
  'tools.diff.original': '原文',
  'tools.diff.modified': '修改后',
  'tools.diff.result': '结果',
  'tools.diff.split': '分栏',
  'tools.diff.unified': '合并',
  'tools.diff.line': '行',
  'tools.diff.word': '词',
  'tools.diff.char': '字',
  'tools.diff.chars': '字符',
  'tools.diff.unchanged': '未变',
  'tools.diff.labelA': 'A',
  'tools.diff.labelB': 'B',
  'tools.diff.clear': '清空',

  // ============================================================
  // tools.string
  // ============================================================
  'tools.string.title': '字符串转换',
  'tools.string.swap': '交换',
  'tools.string.input': '输入',
  'tools.string.output': '输出',
  'tools.string.placeholder': '输入或粘贴要转换的文本...',
  'tools.string.format': '格式',
  'tools.string.mode': '模式',
  'tools.string.direction': '方向',
  'tools.string.encode': '编码',
  'tools.string.decode': '解码',
  'tools.string.tabUnicode': 'Unicode',
  'tools.string.tabHtml': 'HTML',
  'tools.string.tabUrl': 'URL',
  'tools.string.tabNormalize': '标准化',
  'tools.string.tabAscii': 'ASCII',
  'tools.string.tabString': '字符串字面量',
  'tools.string.unicodeEscape': '\\uXXXX 转义',
  'tools.string.unicodeHex': '\\xXX 转义',
  'tools.string.unicodeCode': 'U+XXXX 表示',
  'tools.string.unicodeDecimal': '&#NNNN; 数字',
  'tools.string.unicodeRaw': '原文(直通)',
  'tools.string.htmlNumeric': '数字 (&#NNNN;)',
  'tools.string.htmlHex': '十六进制 (&#xNNNN;)',
  'tools.string.htmlNamed': '命名实体 (&amp;name;)',
  'tools.string.htmlAll': '全部(常见用命名,其它用数字)',
  'tools.string.form': '形式',
  'tools.string.normalizeHelp': '将字符串标准化为四种 Unicode 标准化形式之一。',
  'tools.string.asciiStrip': '去掉非 ASCII',
  'tools.string.asciiReplace': '非 ASCII 替换为 ?',
  'tools.string.asciiEscape': '转义为 \\uXXXX',
  'tools.string.asciiCodepoints': '显示码点',
  'tools.string.sJson': 'JSON 字符串',
  'tools.string.sJs': 'JavaScript 字符串',
  'tools.string.sCss': 'CSS 字符串',
  'tools.string.urlDecodeError': 'URL 解码失败',

  // ============================================================
  // tools.crypto
  // ============================================================
  'tools.crypto.title': '加解密',
  'tools.crypto.aesGcm': 'AES-GCM',
  'tools.crypto.aesCbc': 'AES-CBC',
  'tools.crypto.rsa': 'RSA-OAEP',
  'tools.crypto.rc4': 'RC4',
  'tools.crypto.caesar': '凯撒',
  'tools.crypto.vigenere': '维吉尼亚',
  'tools.crypto.xor': 'XOR',
  'tools.crypto.key': '密钥',
  'tools.crypto.iv': 'IV / Nonce',
  'tools.crypto.aad': '附加认证数据',
  'tools.crypto.shift': '偏移',
  'tools.crypto.plaintext': '明文',
  'tools.crypto.ciphertext': '密文',
  'tools.crypto.encrypt': '加密',
  'tools.crypto.decrypt': '解密',
  'tools.crypto.encryptWithPublic': '用公钥加密',
  'tools.crypto.decryptWithPrivate': '用私钥解密',
  'tools.crypto.generateKeys': '生成密钥对',
  'tools.crypto.publicKey': '公钥',
  'tools.crypto.privateKey': '私钥',
  'tools.crypto.aesKeyPlaceholder': 'Base64 编码的 16/24/32 字节密钥',
  'tools.crypto.aesGcmIvPlaceholder': 'Base64 编码的 12 字节 nonce',
  'tools.crypto.aesCbcIvPlaceholder': 'Base64 编码的 16 字节 IV',
  'tools.crypto.aadPlaceholder': '可选的附加认证数据',
  'tools.crypto.randomKey': '随机密钥',
  'tools.crypto.randomIv': '随机 IV',
  'tools.crypto.aesKeyLengthError': 'AES 密钥必须是 16 / 24 / 32 字节(128 / 192 / 256 位)。',
  'tools.crypto.gcmIvLengthError': 'GCM nonce 必须是 12 字节。',
  'tools.crypto.cbcIvLengthError': 'CBC IV 必须是 16 字节。',
  'tools.crypto.rsaKeySize': '密钥长度',
  'tools.crypto.rsaKeyGenerated': '密钥对已生成 — 用公钥加密,私钥解密。',
  'tools.crypto.rsaKeyMissing': '请先生成密钥对。',
  'tools.crypto.rsaHelp': 'RSA-OAEP + SHA-256。密钥对在浏览器本地生成,不会上传到任何服务器。',
  'tools.crypto.vigenereKeyPlaceholder': 'A–Z 字母(其它字符会被忽略)',
  'tools.crypto.vigenereKeyEmpty': '维吉尼亚密钥必须包含 A–Z 字母。',
  'tools.crypto.xorKeyPlaceholder': '密钥字节(hex 如 deadbeef)或纯文本',
  'tools.crypto.xorKeyHelp': '密钥循环与明文异或。加密 = 解密,对称。',
  'tools.crypto.xorKeyEmpty': 'XOR 密钥不能为空。',
  'tools.crypto.rc4Note': 'RC4 是过时的流密码,新代码不建议使用,这里保留是为了互操作。'
}