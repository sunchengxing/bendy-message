use worker::*;

pub async fn serve_admin(_ctx: RouteContext<()>) -> Result<Response> {
    Response::from_html(include_str!("../../../admin/index.html"))
}
