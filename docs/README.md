# Biliup 文档站

本文档站使用 [Zola](https://www.getzola.org/)，CI 使用 Zola 0.18.0 构建。

## 内容目录与命名

- 用户文档放在 `content/docs/`，按现有 `getting-started/`、`guide/`、`tutorials/`、`help/` 分类。
- 新教程使用小写连字符文件名，例如 `recording-and-masking.md`。页面沿用现有 TOML front matter、`docs/page.html` 模板和 `weight` 排序。
- 优先更新对应页面，不在 `docs/` 根目录增加实施报告、检查清单或快速参考副本。
- 文档内部链接使用 Zola 的 `@/docs/.../page.md` 路径，随站点 base URL 正确生成。
- 图片放在 `static/images/<教程名称>/`；未提供图片时，在正文留下截图说明和目标路径。该目录的 README 记录文件名及截图要求。取得真实图片后再插入图片引用，不使用空图片文件。
- 更新日志继续维护在 [content/docs/guide/CHANGELOG.md](content/docs/guide/CHANGELOG.md)。README、更新日志及主题自带的常规说明文件保留原有用途。

## 当前教程

- [斗鱼 Cookie、自动续期与画质](content/docs/tutorials/douyu-cookie-guide.md)
- [录制分段、画面遮挡与录像合并](content/docs/tutorials/recording-and-masking.md)

## 本地预览

```sh
cd docs
zola serve
```

## 构建检查

在仓库根目录运行：

```sh
cd docs
zola build --drafts
```

输出位于 `docs/public/`。CI 对文档 PR 使用相同的 `--drafts` 参数；新增或修改页面后应检查 front matter、内部链接及渲染结果。
