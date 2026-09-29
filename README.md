# 我的 Aidoku 图源

本仓库准备了 7 个网站的图源。其中 18comic、E-Hentai、Hitomi、nhentai、拷贝漫画、绅士漫画的安装包来自 [Aidoku Community Sources](https://github.com/Aidoku-Community/sources)。Komiic.cc 使用修改后的源码，由 GitHub Actions 编译；它与使用 komiic.com 的旧 Komiic 图源具有不同的 ID。

上传所有文件到公开 GitHub 仓库 `yushansiji233/aidoku` 的根目录后，打开 Actions，等待 **Build Komiic.cc Aidoku source** 成功。工作流成功后，仓库根目录的 `index.min.json` 应当包含 7 个源，`sources/zh.komiiccc-v1.aix` 应当存在。只有这时再向 Aidoku 添加：

`https://cdn.jsdelivr.net/gh/yushansiji233/aidoku@main/index.min.json`

如列表尚只有 6 个源，说明 Komiic.cc 构建尚未成功。安装包在 iPad 上的实际阅读仍需验证。

本上传包不包含哔咔图源。

上游安装包及衍生源码遵循随附的 MIT/Apache 许可证。
