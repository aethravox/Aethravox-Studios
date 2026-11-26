// Lógica de gestión de documentos:
// - Subir archivos (PPTX, DOCX, PDF)
// - Convertir a PDF con LibreOffice headless para previews
// - Generar slugs y rutas seguras

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Result};

/// Tipos de archivo soportados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Pdf,
    Docx,
    Pptx,
    Unknown,
}

impl FileType {
    pub fn from_filename(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.ends_with(".pdf") {
            Self::Pdf
        } else if lower.ends_with(".docx") {
            Self::Docx
        } else if lower.ends_with(".pptx") {
            Self::Pptx
        } else {
            Self::Unknown
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Docx => "docx",
            Self::Pptx => "pptx",
            Self::Unknown => "unknown",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pdf => "📕",
            Self::Docx => "📘",
            Self::Pptx => "📙",
            Self::Unknown => "📄",
        }
    }

    pub fn mime(&self) -> &'static str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            Self::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Self::Unknown => "application/octet-stream",
        }
    }

    /// ¿Necesita conversión a PDF para previsualización?
    pub fn needs_conversion(&self) -> bool {
        matches!(self, Self::Docx | Self::Pptx)
    }
}

/// Comprueba si LibreOffice está disponible en el sistema.
pub fn libreoffice_available() -> bool {
    Command::new("libreoffice")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Convierte un archivo (DOCX/PPTX) a PDF usando LibreOffice headless.
/// El PDF se guarda en `out_dir` con el mismo nombre base pero extensión `.pdf`.
pub fn convert_to_pdf(input: &Path, out_dir: &Path) -> Result<PathBuf> {
    if !input.exists() {
        return Err(anyhow!("archivo de entrada no existe: {:?}", input));
    }
    std::fs::create_dir_all(out_dir)?;

    let output = Command::new("libreoffice")
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(out_dir)
        .arg(input)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("libreoffice falló: {}", stderr));
    }

    // El PDF tendrá el mismo nombre base que el input.
    let stem = input
        .file_stem()
        .ok_or_else(|| anyhow!("no se pudo obtener el nombre base"))?
        .to_string_lossy()
        .to_string();
    let pdf_path = out_dir.join(format!("{}.pdf", stem));

    if !pdf_path.exists() {
        return Err(anyhow!(
            "libreoffice terminó pero no generó el PDF esperado en {:?}",
            pdf_path
        ));
    }

    Ok(pdf_path)
}

/// Limpia un nombre de archivo para usarlo como nombre seguro en disco.
/// Devuelve un nombre sin espacios ni caracteres raros, manteniendo la extensión.
pub fn sanitize_filename(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "documento".into());
    let ext = Path::new(name)
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();

    let clean: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    format!("{}{}", clean, ext)
}

/// Genera un nombre único para el archivo en disco basado en el ID del documento
/// + el nombre saneado. Ej: `42_mipresentacion.pptx`
pub fn disk_path(doc_id: i64, original_name: &str) -> String {
    let clean = sanitize_filename(original_name);
    format!("{}_{}", doc_id, clean)
}

/// Formatea un tamaño de archivo en bytes a un string legible.
pub fn human_size(bytes: i64) -> String {
    const UNITS: &[(&str, i64)] = &[
        ("TB", 1_099_511_627_776),
        ("GB", 1_073_741_824),
        ("MB", 1_048_576),
        ("KB", 1_024),
    ];
    for (unit, threshold) in UNITS {
        if bytes >= *threshold {
            let v = bytes as f64 / *threshold as f64;
            return format!("{:.1} {}", v, unit);
        }
    }
    format!("{} B", bytes)
}
