import vue from 'eslint-plugin-vue'
import ts from 'typescript-eslint'
export default [
  { ignores: ['dist/**', 'src-tauri/target/**'] },
  ...vue.configs['flat/essential'],
  { files: ['**/*.vue'], languageOptions: { parserOptions: { parser: ts.parser } } },
]
