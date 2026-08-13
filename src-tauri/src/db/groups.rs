//! `groups` / `group_members` repository.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::models::{Group, GroupMemberBrief, NewGroupInput};
use crate::state::{new_id, now_ms};

/// Row shape shared by `list`/`get` before the members are joined in.
struct GroupBase {
    id: String,
    name: String,
    topic: String,
    background: String,
    avatar: Option<String>,
    created_at: i64,
}

fn row_to_group_base(r: &Row) -> rusqlite::Result<GroupBase> {
    return Ok(GroupBase {
        id: r.get("id")?,
        name: r.get("name")?,
        topic: r.get("topic")?,
        background: r.get("background")?,
        avatar: r.get::<_, Option<String>>("avatar_path")?,
        created_at: r.get("created_at")?,
    });
}

/// A group's members, ordered by add position.
fn members_of(conn: &Connection, group_id: &str) -> Result<Vec<GroupMemberBrief>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.name, c.avatar_path
             FROM group_members gm
             JOIN characters c ON c.id = gm.character_id
             WHERE gm.group_id = ?1
             ORDER BY gm.position ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![group_id], |r| {
            Ok(GroupMemberBrief {
                id: r.get(0)?,
                name: r.get(1)?,
                avatar: r.get::<_, Option<String>>(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    return rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string());
}

fn to_group(conn: &Connection, base: GroupBase) -> Result<Group, String> {
    let members = members_of(conn, &base.id)?;
    return Ok(Group {
        id: base.id,
        name: base.name,
        topic: base.topic,
        background: base.background,
        avatar: base.avatar,
        members,
        created_at: base.created_at,
    });
}

pub fn list(conn: &Connection) -> Result<Vec<Group>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, topic, background, avatar_path, created_at
             FROM groups ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let bases = stmt
        .query_map([], row_to_group_base)
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    return bases.into_iter().map(|b| to_group(conn, b)).collect();
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Group>, String> {
    let base = conn
        .query_row(
            "SELECT id, name, topic, background, avatar_path, created_at
             FROM groups WHERE id = ?1",
            params![id],
            row_to_group_base,
        )
        .optional()
        .map_err(|e| e.to_string())?;
    return match base {
        None => Ok(None),
        Some(b) => to_group(conn, b).map(Some),
    };
}

/// Create a group with its members. Requires a non-empty name and at least
/// two members.
pub fn insert(conn: &Connection, avatars_dir: &Path, input: NewGroupInput) -> Result<Group, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("Group name is required".into());
    }
    if input.member_ids.len() < 2 {
        return Err("A group needs at least two members".into());
    }

    let id = new_id();
    let ts = now_ms();

    let avatar_path = match input.avatar.as_deref() {
        Some(data) if !data.is_empty() => Some(super::save_avatar(avatars_dir, &id, data)?),
        _ => None,
    };

    conn.execute(
        "INSERT INTO groups (id, name, topic, background, avatar_path, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, name, input.topic, input.background, avatar_path, ts],
    )
    .map_err(|e| e.to_string())?;

    for (i, character_id) in input.member_ids.iter().enumerate() {
        conn.execute(
            "INSERT INTO group_members (group_id, character_id, position) VALUES (?1, ?2, ?3)",
            params![id, character_id, i as i64],
        )
        .map_err(|e| e.to_string())?;
    }

    return get(conn, &id)?.ok_or_else(|| format!("Group '{id}' not found after insert"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NewCharacterInput;

    fn open_test_db() -> (crate::db::Db, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("kiwi-groups-test-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.db");
        let avatars_dir = dir.join("avatars");
        let db = crate::db::Db::open(&db_path, avatars_dir).unwrap();
        return (db, dir);
    }

    fn make_character(conn: &Connection, name: &str) -> String {
        let c = crate::db::characters::insert(
            conn,
            std::path::Path::new("."),
            NewCharacterInput {
                name: name.into(),
                info: String::new(),
                appearance: String::new(),
                description: String::new(),
                initial_message: String::new(),
                avatar: None,
            },
        )
        .unwrap();
        return c.id;
    }

    #[test]
    fn insert_requires_two_members() {
        let (db, dir) = open_test_db();
        let a = make_character(&db.conn, "GroupTestA");
        let err = insert(
            &db.conn,
            &db.avatars_dir,
            NewGroupInput {
                name: "Solo".into(),
                topic: String::new(),
                background: String::new(),
                avatar: None,
                member_ids: vec![a],
            },
        )
        .unwrap_err();
        assert!(err.contains("at least two"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn insert_and_list_roundtrip_preserves_member_order() {
        let (db, dir) = open_test_db();
        let a = make_character(&db.conn, "GroupTestA");
        let b = make_character(&db.conn, "GroupTestB");

        let created = insert(
            &db.conn,
            &db.avatars_dir,
            NewGroupInput {
                name: "B-Day Party".into(),
                topic: "A birthday celebration".into(),
                background: "Dina invited everyone".into(),
                avatar: None,
                member_ids: vec![b.clone(), a.clone()], // deliberately reversed order
            },
        )
        .unwrap();

        assert_eq!(created.name, "B-Day Party");
        assert_eq!(created.members.len(), 2);
        // Order must match the order members were added (b, then a), not insertion id order.
        assert_eq!(created.members[0].id, b);
        assert_eq!(created.members[1].id, a);

        let listed = list(&db.conn).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);
        assert_eq!(listed[0].members.len(), 2);

        std::fs::remove_dir_all(&dir).ok();
    }
}
