module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    // 类型规则
    'type-enum': [
      2,
      'always',
      [
        'feat',     // 新功能
        'fix',      // 修复bug
        'docs',     // 文档更新
        'style',    // 代码格式调整
        'refactor', // 代码重构
        'perf',     // 性能优化
        'test',     // 测试相关
        'chore',    // 构建过程或辅助工具
        'ci',       // CI/CD相关
        'build',    // 构建相关
        'revert',   // 回滚
      ],
    ],
    
    // 主题规则
    'subject-case': [2, 'never', ['pascal-case', 'upper-case']],
    'subject-empty': [2, 'never'],
    'subject-full-stop': [2, 'never', '.'],
    'subject-max-length': [2, 'always', 50],
    'subject-min-length': [2, 'always', 10],
    
    // 范围规则（可选）
    'scope-case': [2, 'always', 'lower-case'],
    'scope-enum': [
      2,
      'always',
      [
        // 前端相关
        'ui', 'component', 'view', 'style', 'router',
        // 后端相关
        'api', 'service', 'model', 'config',
        // 项目特定
        'image', 'chunk', 'cache', 'webgl', 'tauri',
        // 通用
        'deps', 'ci', 'build', 'docs', 'test',
      ],
    ],
    
    // 正文规则
    'body-max-line-length': [2, 'always', 72],
    'body-leading-blank': [2, 'always'],
    'body-min-length': [2, 'always', 20],
    
    // 页脚规则
    'footer-leading-blank': [2, 'always'],
    'footer-max-line-length': [2, 'always', 72],
    
    // 其他规则
    'header-max-length': [2, 'always', 100],
    'type-case': [2, 'always', 'lower-case'],
    'type-empty': [2, 'never'],
  },
  
  // 自定义解析器
  parserPreset: {
    parserOpts: {
      headerPattern: /^(\w*)(?:\(([^)]*)\))?: (.*)$/,
      headerCorrespondence: ['type', 'scope', 'subject'],
    },
  },
  
  // 帮助信息
  helpUrl: 'https://github.com/conventional-changelog/commitlint/#what-is-commitlint',
};
