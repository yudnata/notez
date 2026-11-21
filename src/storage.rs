use crate::model::Workspace;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

pub fn default_path() -> PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--data-dir" {
            if let Some(dir) = args.next() {
                return PathBuf::from(dir).join("workspace.notvas");
            }
        }
    }
    let root = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    root.join("Notvas").join("workspace.notvas")
}

pub fn load(path: &Path) -> Result<Workspace, String> {
    let bytes =
        fs::read(path).map_err(|e| format!("Tidak bisa membaca {}: {e}", path.display()))?;
    let mut workspace: Workspace = serde_json::from_slice(&bytes)
        .map_err(|e| format!("File tidak valid: {e}. File asli tidak diubah."))?;
    workspace.validate()?;
    Ok(workspace)
}

pub fn unused_path(folder: &Path) -> PathBuf {
    let base = folder.join("workspace.notvas");
    if !base.exists() {
        return base;
    }
    let mut index = 1_u64;
    loop {
        let path = folder.join(format!("workspace-{index}.notvas"));
        if !path.exists() {
            return path;
        }
        index += 1;
    }
}

pub fn save(path: &Path, workspace: &Workspace) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(workspace).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| format!("Tidak bisa membuat folder: {e}"))?;
    }
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| -> std::io::Result<()> {
        let mut file = File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        if path.exists() {
            let backup = path.with_extension("notvas.bak");
            let _ = fs::remove_file(&backup);
            if fs::rename(path, &backup).is_err() {
                fs::copy(path, &backup)?;
            }
        }
        if let Err(err) = fs::rename(&temporary, path) {
            let backup = path.with_extension("notvas.bak");
            if !path.exists() && backup.exists() {
                let _ = fs::rename(&backup, path);
            }
            return Err(err);
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "Gagal menyimpan: {e}. Perubahan masih ada di jendela ini."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Kind, Task, TextStyle};

    #[test]
    fn roundtrip_all_modes_backup_and_corrupt_file() {
        let dir = std::env::temp_dir().join(format!(
            "notvas-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("workspace.notvas");
        let mut ws = Workspace::default();
        ws.add(Kind::Note);
        ws.documents[0].body.reconcile(
            "Halo 猫".into(),
            TextStyle {
                bold: true,
                ..Default::default()
            },
        );
        for layout in [
            crate::model::ImageLayout::Text,
            crate::model::ImageLayout::Overlay,
        ] {
            let body = &mut ws.documents[0].body;
            body.insert_image(
                body.text.chars().count(),
                crate::model::test_image(),
                "foto.png".into(),
                TextStyle::default(),
            );
            let image = body.images.last_mut().unwrap();
            image.layout = layout;
            image.offset = [20.0, 10.0];
        }
        ws.add(Kind::Checklist);
        ws.add(Kind::Kanban);
        ws.add(Kind::Canvas);
        ws.documents[0].note_position = Some([130.0, 240.0]);
        let mut shape = crate::canvas::model::Element::new(
            1,
            crate::canvas::model::ElementKind::Rectangle,
            [40.0, 60.0],
            crate::canvas::model::Style::default(),
        );
        shape.text = "Shape tersimpan".into();
        ws.documents[3].canvas.elements.push(shape);
        ws.documents[3].canvas.next_id = 2;
        let id = ws.id();
        ws.documents[2].tasks.push(Task {
            id,
            text: "Tugas".into(),
            detail: "Rincian".into(),
            done: false,
            column: 2,
        });
        save(&path, &ws).unwrap();
        assert_eq!(load(&path).unwrap(), ws);
        let recovered_path = unused_path(&dir);
        assert_ne!(recovered_path, path);
        assert!(!recovered_path.exists());
        ws.documents[0].title = "Diubah".into();
        save(&path, &ws).unwrap();
        assert_ne!(load(&path.with_extension("notvas.bak")).unwrap(), ws);
        fs::write(&path, "rusak").unwrap();
        assert!(load(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "rusak");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn legacy_notes_without_board_fields_still_load() {
        let mut workspace = Workspace::default();
        workspace.add(Kind::Note);
        let mut value = serde_json::to_value(&workspace).unwrap();
        let doc = value["documents"][0].as_object_mut().unwrap();
        doc.remove("canvas");
        doc.remove("note_color");
        doc.remove("note_position");
        doc["body"].as_object_mut().unwrap().remove("images");
        let mut restored: Workspace = serde_json::from_value(value).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.documents[0].title, "Catatan baru");
        assert!(restored.documents[0].canvas.elements.is_empty());
        assert!(restored.documents[0].body.images.is_empty());
    }
}
