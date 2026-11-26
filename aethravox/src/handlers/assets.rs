// Visor PDF: viewer.html es una plantilla Askama (templates/viewer/viewer.html),
// renderizada por el handler viewer() igual que cualquier otra página. La
// librería third-party PDF.js (pdf.js + pdf.worker.js) vive en static/pdf/ y la
// sirve ServeDir como vendor JS normal — no se embebe ni se rutea a mano.
//
// El visor es público porque se carga tanto desde /docs/:id (con sesión) como
// desde /d/:uuid (vista pública sin sesión).

use askama::Template;
use axum::response::IntoResponse;

/// Plantilla Askama del visor PDF (iframe standalone; lee ?file= por JS).
#[derive(Template)]
#[template(path = "herramientas/viewer/viewer.html")]
pub struct ViewerTemplate {}

/// GET /pdf-viewer/viewer.html — renderiza la plantilla Askama del visor.
pub async fn viewer() -> impl IntoResponse {
    ViewerTemplate {}
}
