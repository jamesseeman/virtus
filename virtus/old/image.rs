use crate::Connection;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Image {
    id: Uuid,
    installer: bool,
    filename: String,
}

impl Image {
    pub fn new(filename: String, installer: bool, conn: &Connection) -> Result<Self> {
        let image = Self {
            id: Uuid::new_v4(),
            filename,
            installer,
        };
        conn.db
            .open_tree("images")?
            .insert(image.id, bincode::serialize(&image)?)?;
        Ok(image)
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_filename(&self) -> String {
        self.filename.clone()
    }

    pub fn is_installer(&self) -> bool {
        self.installer
    }

    pub fn get(id: &Uuid, conn: &Connection) -> Result<Option<Self>> {
        match conn.db.open_tree("images")?.get(id)? {
            Some(image) => Ok(Some(bincode::deserialize(&image)?)),
            None => Ok(None),
        }
    }

    pub fn list(conn: &Connection) -> Result<Vec<Uuid>> {
        let images: Vec<Uuid> = conn
            .db
            .open_tree("images")?
            .into_iter()
            .filter_map(|result| result.ok())
            .filter_map(|option| Uuid::from_slice(&option.0).ok())
            .collect();

        Ok(images)
    }

    pub fn delete_by_id(id: Uuid, conn: &Connection) -> Result<()> {
        conn.db.open_tree("images")?.remove(id)?;
        Ok(())
    }

    pub fn delete(self, conn: &Connection) -> Result<()> {
        conn.db.open_tree("images")?.remove(self.id)?;
        Ok(())
    }
}
