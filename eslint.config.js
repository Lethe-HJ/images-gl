import js from '@eslint/js';
import typescript from '@typescript-eslint/eslint-plugin';
import typescriptParser from '@typescript-eslint/parser';
import vue from 'eslint-plugin-vue';
import vueParser from 'vue-eslint-parser';

export default [
  js.configs.recommended,
  {
    files: ['**/*.{js,ts,vue}'],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: typescriptParser,
        ecmaVersion: 2022,
        sourceType: 'module',
      },
      globals: {
        // Browser globals
        window: 'readonly',
        document: 'readonly',
        console: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        HTMLCanvasElement: 'readonly',
        WebGL2RenderingContext: 'readonly',
        WebGLProgram: 'readonly',
        WebGLVertexArrayObject: 'readonly',
        WebGLBuffer: 'readonly',
        WebGLTexture: 'readonly',
        HTMLImageElement: 'readonly',
        ImageBitmap: 'readonly',
        WebGLRenderingContext: 'readonly',
        WebGLShader: 'readonly',
        HTMLScriptElement: 'readonly',
        HTMLElement: 'readonly',
        HTMLInputElement: 'readonly',

        // Node.js globals
        process: 'readonly',
        Buffer: 'readonly',
        __dirname: 'readonly',
        __filename: 'readonly',
        global: 'readonly',
        module: 'readonly',
        require: 'readonly',
        exports: 'readonly',
      },
    },
    plugins: {
      '@typescript-eslint': typescript,
      vue,
    },
    rules: {
      // TypeScript 规则 - 使用官方推荐
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrorsIgnorePattern: '^_',
          // 忽略枚举值，因为它们可能被外部使用
          ignoreRestSiblings: true,
        },
      ],
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-non-null-assertion': 'error',
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/explicit-module-boundary-types': 'off',

      // Vue 规则 - 使用官方推荐
      'vue/multi-word-component-names': 'off',
      'vue/no-v-html': 'warn',
      'vue/require-default-prop': 'off',
      'vue/require-explicit-emits': 'warn',

      // 通用规则 - 严格标准
      'no-console': process.env.NODE_ENV === 'production' ? 'warn' : 'off',
      'no-debugger': process.env.NODE_ENV === 'production' ? 'warn' : 'off',
      // 禁用通用的 no-unused-vars，使用 TypeScript 版本
      'no-unused-vars': 'off',
      'prefer-const': 'error',
      'no-var': 'error',
      'object-shorthand': 'error',
      'prefer-template': 'error',
    },
  },
  {
    files: ['**/*.vue'],
    ...vue.configs['vue3-recommended'],
  },
  {
    ignores: [
      'dist/',
      'node_modules/',
      'src-tauri/target/',
      'src-tauri/chunk_cache/',
      '*.d.ts',
      'scripts/**/*.js',
      'scripts/**/*.cjs',
      'src/scripts/**/*.js',
      'src/scripts/**/*.cjs',
    ],
  },
];
