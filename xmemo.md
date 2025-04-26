# 開発メモ

ojの使い方

- <https://github.com/online-judge-tools/oj/blob/master/README.ja.md>
- `oj test -c "cargo run --bin abc300 a`
  
AtCoder CLIの使い方

- `acc new abc100`

テンプレートを作成

```bash
cd `acc config-dir`
mkdir <your-template-name>
cd <your-template-name>
vim template.json # write your template settings
```

テンプレートの適応

```bash
acc templates
acc new|add --template <your-template-name>
acc config default-template <your-template-name>
```
