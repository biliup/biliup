use axum::http::{StatusCode, Uri, header};
use axum::response::{Html, IntoResponse, Response};
use rust_embed::Embed;

/// 默认首页文件名
static INDEX_HTML: &str = "index.html";

/// 嵌入的静态资源
#[derive(Embed)]
#[folder = "../../out/"]
struct Assets;

/// 静态文件处理器，用于服务单页应用
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // 根路径或直接访问index.html时返回首页
    if path.is_empty() || path == INDEX_HTML {
        return index_html().await;
    }

    // 尝试查找对应的HTML文件
    let guess_html = path.to_owned() + ".html";
    if let Some(html) = Assets::get(&guess_html) {
        return Html(html.data).into_response();
    }

    // 查找静态资源文件
    match Assets::get(path) {
        Some(content) => {
            // 根据文件扩展名推断MIME类型
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            // 文件不存在，返回404
            not_found().await
        }
    }
}

/// 返回首页HTML内容
async fn index_html() -> Response {
    match Assets::get(INDEX_HTML) {
        Some(content) => Html(content.data).into_response(),
        None => not_found().await,
    }
}

/// 返回404错误响应
async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "404").into_response()
}
