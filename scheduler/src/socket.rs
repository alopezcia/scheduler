//! Servidor del socket Unix de control. Solo hay un cliente esperado (el backend), pero se
//! aceptan varias conexiones para que un reinicio del backend no tenga que esperar a que
//! se libere la anterior.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;

use tokio::net::UnixListener;

use crate::control::{Control, serve_connection};

pub async fn serve(path: &Path, control: Arc<Control>) -> std::io::Result<()> {
    // Un socket de una ejecución anterior que no se limpió impide el bind.
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;

    // El socket da control total sobre las ejecuciones: solo el propietario.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;

    tracing::info!("Socket de control escuchando en {}", path.display());

    loop {
        let (stream, _) = listener.accept().await?;
        let control = control.clone();
        tokio::spawn(async move {
            serve_connection(stream, &control).await;
        });
    }
}
