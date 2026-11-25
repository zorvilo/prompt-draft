# 发布与应用内更新

应用通过 Tauri 官方 Updater 检查更新、下载验签并安装 NSIS 更新包，更新入口位于「设置 → 关于」。目前只生成 Windows x86_64 更新清单。源码、发行版安装包和更新清单统一维护在公开仓库 `https://gitee.com/liuliang6666/prompt-draft`。

## 首次配置

公钥已写入 `src-tauri/tauri.conf.json`。更新私钥与密码保存在仓库外 `%USERPROFILE%\.prompt-draft-signing`，不要提交、打印或发到聊天。将 `updater.key` 的完整内容和 `updater-password.txt` 中的密码保存到 Bitwarden 安全笔记，并验证另一台设备能读取。公钥也建议一同备份。确认保险箱保存成功后再处理本地副本；不要在未备份前删除密钥。所有后续版本须使用同一把私钥，不能每次重新生成。

## 发布步骤

1. 同步修改 `package.json`、`src-tauri/Cargo.toml` 和 `src-tauri/tauri.conf.json` 的版本号，更新对应锁文件。运行项目检查。只有明确需要发布且版本间数据库结构兼容时才发布应用内更新；当前不支持旧结构迁移，不能靠自动更新清空用户数据。
2. 从 Bitwarden 将私钥临时保存至仓库外文件，在本次 PowerShell 会话设置 `TAURI_SIGNING_PRIVATE_KEY` 为该文件的绝对路径。通过 `Read-Host -AsSecureString` 获取密码并临时注入 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，不要把密码写进脚本、命令历史或仓库。
3. 运行 `npm run tauri build`，生成 `src-tauri/target/release/bundle/nsis` 下的安装包和 `.sig`。签名用于更新包校验，与 Windows Authenticode 证书不同。
4. 在源码仓库的[发行版页面](https://gitee.com/liuliang6666/prompt-draft/releases)创建对应版本的发行版，上传安装包。用未登录的请求验证附件可以直接下载，不返回网页、登录或验证码；浏览器能手动下载不代表更新程序可以直接下载。
5. 将更新说明保存为 UTF-8 文本，在源码目录执行：

   ```powershell
   node scripts/prepare-update.mjs "安装包的HTTPS直链" "安装包.sig的路径" "说明.txt" "latest.json"
   ```

6. 将生成的 `latest.json` 提交并推送到当前源码仓库的 `master` 分支根目录，确认 `https://gitee.com/liuliang6666/prompt-draft/raw/master/latest.json` 无需登录即可返回 JSON。始终先上传安装包，最后发布清单；清单内签名取自当前安装包，不可复用旧签名。不将任何私钥或密钥密码上传到仓库。
7. 用旧版实际测试检查、下载、验签、安装及重启，确认草稿、供应商密钥、设置仍存在。无更新、网络失败、下载失败、签名错误、生成期间拒绝安装及保存失败均应测试。
8. 完成后清除会话中的签名环境变量和临时私钥文件，保留 Bitwarden 与独立加密备份。

初始未发布 `latest.json` 时，检查更新会显示失败；不能标成已是最新。现有 0.1.0 旧安装没有更新入口，需要手动安装一次带更新功能的版本。版本号相同不会被识别为更新。
