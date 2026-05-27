pub fn serve_admin() -> Result<vercel_runtime::Response<vercel_runtime::ResponseBody>, vercel_runtime::Error> {
    let html = include_str!("../../../admin/index.html");
    vercel_runtime::Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(vercel_runtime::ResponseBody::from(html.to_string()))
        .map_err(|e| vercel_runtime::Error::from(e.to_string()))
}
