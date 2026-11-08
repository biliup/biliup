//! Shared, transactional undo cursor. Restores are new revisions, so in-flight edits stay stale.
use super::super::{
    self as editor, ConnectionPool, Document, Error, Result, Saved, agent::store::Journal,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, Sqlite, Transaction};

const MAX_STEPS: usize = 1000;
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Undo,
    Redo,
    Jump,
}
impl Action {
    pub fn kind(self) -> &'static str {
        match self {
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::Jump => "jump",
        }
    }
    pub fn label(self, target: i64) -> String {
        match self {
            Self::Undo => "撤销".into(),
            Self::Redo => "重做".into(),
            Self::Jump => format!("恢复版本 {target}"),
        }
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_revision: Option<i64>,
    pub redo_revision: Option<i64>,
    pub branch_revision: i64,
}
pub struct Cursor {
    head: i64,
    branch: i64,
    past: Vec<i64>,
    future: Vec<i64>,
}
impl Cursor {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow, expected: i64) -> Result<Self> {
        let cursor = Self {
            head: row.try_get("head_revision")?,
            branch: row.try_get("branch_revision")?,
            past: serde_json::from_str(row.try_get("past_json")?)?,
            future: serde_json::from_str(row.try_get("future_json")?)?,
        };
        if cursor.head != expected
            || cursor
                .past
                .iter()
                .chain(cursor.future.iter())
                .any(|rev| *rev < 1 || *rev >= expected)
        {
            return Err(Error::Invalid("工程历史游标无效，请重新加载工程".into()));
        }
        Ok(cursor)
    }
    pub fn info(&self) -> Info {
        Info {
            can_undo: !self.past.is_empty(),
            can_redo: !self.future.is_empty(),
            undo_revision: self.past.last().copied(),
            redo_revision: self.future.last().copied(),
            branch_revision: self.branch,
        }
    }
}
fn push(stack: &mut Vec<i64>, revision: i64) {
    stack.push(revision);
    if stack.len() > MAX_STEPS {
        stack.drain(..stack.len() - MAX_STEPS);
    }
}

pub(crate) async fn resolve(
    tx: &mut Transaction<'_, Sqlite>,
    project: i64,
    mut revision: i64,
) -> Result<i64> {
    loop {
        let row=sqlx::query("SELECT parent_revision,is_alias FROM editor_history_links WHERE project_id=? AND revision=?").bind(project).bind(revision).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
        if !row.try_get::<bool, _>("is_alias")? {
            return Ok(revision);
        }
        let parent: i64 = row.try_get("parent_revision")?;
        if parent < 1 || parent >= revision {
            return Err(Error::Invalid("工程历史引用无效".into()));
        }
        revision = parent;
    }
}

pub async fn view(
    pool: &ConnectionPool,
    project: i64,
    revision: i64,
) -> Result<(i64, std::collections::HashSet<i64>)> {
    let rows=sqlx::query("WITH RECURSIVE lineage(revision,parent_revision,is_alias) AS (SELECT revision,parent_revision,is_alias FROM editor_history_links WHERE project_id=? AND revision=? UNION ALL SELECT l.revision,l.parent_revision,l.is_alias FROM editor_history_links l JOIN lineage p ON l.revision=p.parent_revision WHERE l.project_id=? AND l.revision<p.revision) SELECT revision,is_alias FROM lineage ORDER BY revision DESC")
        .bind(project).bind(revision).bind(project).fetch_all(pool).await?;
    let content = rows
        .iter()
        .find(|row| !row.get::<bool, _>("is_alias"))
        .ok_or(Error::NotFound)?
        .get::<i64, _>("revision");
    Ok((
        content,
        rows.iter().map(|row| row.get("revision")).collect(),
    ))
}

/// Called for every save, including assistant/import/capture saves, inside the existing CAS transaction.
pub async fn advance(
    tx: &mut Transaction<'_, Sqlite>,
    project: i64,
    previous: Option<i64>,
    revision: i64,
    journal: Option<&Journal<'_>>,
) -> Result<Info> {
    let mut cursor = if let Some(previous) = previous {
        let row = sqlx::query("SELECT * FROM editor_history_cursor WHERE project_id=?")
            .bind(project)
            .fetch_one(&mut **tx)
            .await?;
        Cursor::from_row(&row, previous)?
    } else {
        Cursor {
            head: revision,
            branch: revision,
            past: vec![],
            future: vec![],
        }
    };
    let mut parent = previous;
    let mut alias = false;
    if let Some(previous) = previous {
        match journal {
            Some(Journal::Move { action, target }) => {
                if *target < 1 || *target > previous {
                    return Err(Error::Invalid("历史目标版本无效".into()));
                }
                match action {
                    Action::Undo => {
                        if cursor.past.pop() != Some(*target) {
                            return Err(Error::Conflict);
                        }
                        push(&mut cursor.future, previous);
                    }
                    Action::Redo => {
                        if cursor.future.pop() != Some(*target) {
                            return Err(Error::Conflict);
                        }
                        push(&mut cursor.past, previous);
                    }
                    Action::Jump => {
                        push(&mut cursor.past, previous);
                        cursor.future.clear();
                    }
                }
                parent = Some(resolve(tx, project, *target).await?);
                alias = true;
            }
            Some(Journal::Undo { turn }) => {
                let target: i64 = sqlx::query_scalar(
                    "SELECT base_revision FROM editor_agent_turns WHERE project_id=? AND id=?",
                )
                .bind(project)
                .bind(turn)
                .fetch_one(&mut **tx)
                .await?;
                parent = Some(resolve(tx, project, target).await?);
                alias = true;
                push(&mut cursor.past, previous);
                cursor.future.clear();
            }
            other => {
                // One uninterrupted assistant turn is one undo step, as in the reference editor.
                let same_turn = if let Some(Journal::Edit { turn, .. }) = other {
                    sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM editor_revision_notes WHERE project_id=? AND revision=? AND author='agent' AND kind='edit' AND turn_id=?)")
                        .bind(project).bind(previous).bind(turn).fetch_one(&mut **tx).await?
                } else {
                    false
                };
                if !same_turn {
                    push(&mut cursor.past, previous)
                }
                cursor.future.clear();
            }
        }
    }
    sqlx::query("INSERT INTO editor_history_links(project_id,revision,parent_revision,is_alias) VALUES(?,?,?,?)").bind(project).bind(revision).bind(parent).bind(alias).execute(&mut **tx).await?;
    cursor.head = revision;
    if alias {
        cursor.branch = revision;
    }
    sqlx::query("INSERT INTO editor_history_cursor(project_id,head_revision,branch_revision,past_json,future_json) VALUES(?,?,?,?,?) ON CONFLICT(project_id) DO UPDATE SET head_revision=excluded.head_revision,branch_revision=excluded.branch_revision,past_json=excluded.past_json,future_json=excluded.future_json")
        .bind(project).bind(revision).bind(cursor.branch).bind(serde_json::to_string(&cursor.past)?).bind(serde_json::to_string(&cursor.future)?).execute(&mut **tx).await?;
    Ok(cursor.info())
}

pub async fn step(
    pool: &ConnectionPool,
    project: i64,
    revision: i64,
    action: Action,
    target: Option<i64>,
) -> Result<Saved> {
    let saved = editor::get(pool, project).await?;
    if saved.summary.revision != revision {
        return Err(Error::Conflict);
    }
    let target = match action {
        Action::Undo => saved.history.undo_revision,
        Action::Redo => saved.history.redo_revision,
        Action::Jump => Some(
            target
                .filter(|r| *r > 0 && *r <= revision)
                .ok_or_else(|| Error::Invalid("请选择有效的历史版本".into()))?,
        ),
    };
    let Some(target) = target else {
        return Ok(saved);
    };
    if target == revision {
        return Ok(saved);
    }
    if matches!(action, Action::Jump) {
        let mut tx = pool.begin().await?;
        let unchanged = resolve(&mut tx, project, revision).await?
            == resolve(&mut tx, project, target).await?;
        tx.commit().await?;
        if unchanged {
            return Ok(saved);
        }
    }
    let document = restore_document(
        &saved.document,
        editor::revision(pool, project, target).await?,
    );
    editor::save_agent(
        pool,
        (project, revision),
        document,
        Journal::Move { action, target },
    )
    .await
}

/// Session binding belongs to the project. Restoring its editable contents must not rebind it.
pub fn restore_document(current: &Document, target: Document) -> Document {
    Document {
        project: target.project,
        sources: target.sources,
        ..current.clone()
    }
}
