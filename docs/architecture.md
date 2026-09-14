# To‑Do List Application – Architecture & Implementation Plan

## Overview

This document describes the structure, technology stack, and implementation steps for a simple **To‑Do List** web application.  It is designed to be executed in a separate Codex session (or by a developer) without requiring additional context.

- **Backend** – Rust + Axum, REST API, SQLite storage
- **Frontend** – Astro + TypeScript + Solid‑JS, fetches the API
- **Database** – SQLite with a single `tasks` table and an auto‑generated migration

The goal is a fully functional CRUD application that can be built locally and optionally deployed.

## Directory Layout

```
/                      # root of the repo
├── server/              # Rust Axum backend
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── db/
│   │   │   ├── mod.rs          # DB pool + migrations
│   │   │   └── schema.rs        # optional for schema types
│   │   ├── models/
│   │   │   ├── mod.rs
│   │   │   └── task.rs         # Task struct
│   │   └── routes/
│   │       ├── mod.rs
│   │       ├── health.rs
│   │       └── tasks.rs
│   └── migrations/
│       └── 20230913000000_create_tasks_table.sql
├── web/                 # Astro + TS frontend
│   ├── package.json
│   ├── astro.config.mjs
│   ├── src/
│   │   ├── lib/
│   │   │   ├── api.ts
│   │   │   └── types.ts
│   │   ├── routes/
│   │   │   └── index.astro
│   │   ├── components/
│   │   │   ├── TaskItem.astro
│   │   │   └── TaskForm.astro
│   │   └── styles/
│   │       └── globals.css
│   └── public/
│       └── favicon.svg
├── .gitignore
└── README.md
```

## Database Schema

The SQLite migration file creates a single `tasks` table:

```sql
CREATE TABLE tasks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT NOT NULL,
    description TEXT,
    completed   BOOLEAN NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Trigger to keep `updated_at` current
CREATE TRIGGER set_updated_at
AFTER UPDATE ON tasks
FOR EACH ROW
BEGIN
    UPDATE tasks SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;
```

- `id`: Auto‑increment integer.
- `title`: Required string.
- `description`: Optional longer text.
- `completed`: Boolean flag.
- `created_at` / `updated_at`: Timestamps.

## Backend (Rust + Axum)

### Dependencies
Add the following to `server/Cargo.toml`:

```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-native-tls", "macros"] }
tower-http = { version = "0.5", features = ["cors"] }
dotenvy = "0.15"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "json"] }
```

### Database Layer (`server/src/db/mod.rs`)

```rust
use sqlx::SqlitePool;
use tracing::info;

pub async fn init_db() -> anyhow::Result<SqlitePool> {
    let pool = SqlitePool::connect("sqlite://tasks.db").await?;
    sqlx::migrate!().run(&pool).await?;
    info!("Database initialized");
    Ok(pool)
}
```

### Models (`server/src/models/task.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub created_at: String,
    pub updated_at: String,
}
```

### Routes
- **`/api/tasks`** – CRUD endpoints.
- **`/api/health`** – health check.

#### `server/src/routes/tasks.rs`

```rust
use axum::{extract::{Extension, Path, Json}, http::StatusCode, response::IntoResponse, routing::{get, post, put, delete}, Router};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use crate::models::task::Task;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(list).post(create))
        .route("/:id", put(update).delete(delete))
}

async fn list(Extension(pool): Extension<Arc<SqlitePool>>) -> Result<impl IntoResponse, StatusCode> {
    let tasks = sqlx::query_as::<_, Task>("SELECT * FROM tasks")
        .fetch_all(&*pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(tasks))
}

async fn create(
    Extension(pool): Extension<Arc<SqlitePool>>,
    Json(payload): Json<CreateTask>,
) -> Result<impl IntoResponse, StatusCode> {
    let rec = sqlx::query_as::<_, Task>(
        "INSERT INTO tasks (title, description) VALUES (?1, ?2) RETURNING *",
    )
    .bind(&payload.title)
    .bind(&payload.description)
    .fetch_one(&*pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(rec)))
}

async fn update(
    Extension(pool): Extension<Arc<SqlitePool>>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateTask>,
) -> Result<impl IntoResponse, StatusCode> {
    // Build dynamic query – for brevity, omitted complex builder
    let res = sqlx::query("UPDATE tasks SET title = COALESCE(?1, title), description = COALESCE(?2, description), completed = COALESCE(?3, completed) WHERE id = ?4")
        .bind(payload.title)
        .bind(payload.description)
        .bind(payload.completed)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if res.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let updated = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?1")
        .bind(id)
        .fetch_one(&*pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(updated))
}

async fn delete(
    Extension(pool): Extension<Arc<SqlitePool>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = sqlx::query("DELETE FROM tasks WHERE id = ?1")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if res.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
```

#### `server/src/routes/health.rs`

```rust
use axum::{routing::get, Router, Json};

pub fn router() -> Router {
    Router::new().route("/", get(|| async { Json(serde_json::json!({"status":"ok"})) }))
}
```

#### `server/src/routes/mod.rs`

```rust
mod tasks;
mod health;

pub fn router() -> axum::Router {
    axum::Router::new()
        .nest("/tasks", tasks::router())
        .nest("/", health::router())
}
```

#### `server/src/main.rs`

```rust
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = crate::db::init_db().await?;
    let shared_pool = Arc::new(pool);

    let app = Router::new()
        .nest("/api", crate::routes::router())
        .layer(axum::AddExtensionLayer::new(shared_pool))
        .layer(tower_http::cors::CorsLayer::new());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("🚀 Server listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
```

## Frontend (Astro + TypeScript)

### Dependencies (`web/package.json`)

```json
{
  "name": "web",
  "version": "0.1.0",
  "scripts": {
    "dev": "astro dev",
    "build": "astro build",
    "preview": "astro preview"
  },
  "dependencies": {
    "@astrojs/solid-js": "^1.0.0",
    "@astrojs/tailwind": "^3.0.0",
    "axios": "^1.6.0",
    "solid-js": "^1.7.0",
    "solid-js/web": "^1.7.0"
  }
}
```

### API wrapper (`web/src/lib/api.ts`)

```ts
import axios from "axios";
import type { Task } from "./types";

const api = axios.create({ baseURL: "/api" });

export const getTasks = async (): Promise<Task[]> => {
  const { data } = await api.get("/tasks");
  return data;
};

export const createTask = async (title: string, desc?: string): Promise<Task> => {
  const { data } = await api.post("/tasks", { title, description: desc });
  return data;
};

export const updateTask = async (id: number, updates: Partial<Task>): Promise<Task> => {
  const { data } = await api.put(`/tasks/${id}`, updates);
  return data;
};

export const deleteTask = async (id: number): Promise<void> => {
  await api.delete(`/tasks/${id}`);
};
```

### Types (`web/src/lib/types.ts`)

```ts
export interface Task {
  id: number;
  title: string;
  description?: string;
  completed: boolean;
  created_at: string;
  updated_at: string;
}
```

### UI Components
- **`TaskItem.astro`** – renders a single task with a checkbox and delete button.
- **`TaskForm.astro`** – form for creating new tasks.
- **`index.astro`** – main page that pulls data and renders the list.

#### `TaskItem.astro`

```astro
---
import type { Task } from '../lib/types';
const { task, onToggle, onDelete } = Astro.props;
---
<li class="task-item">
  <input type="checkbox" checked={task.completed} onChange={onToggle} />
  <span class={task.completed ? 'completed' : ''}>{task.title}</span>
  <button onClick={onDelete}>✕</button>
</li>
```

#### `TaskForm.astro`

```astro
---
import { createSignal } from "solid-js";
const { onSubmit } = Astro.props;
const [title, setTitle] = createSignal("");
const [desc, setDesc] = createSignal("");

const handleSubmit = async (e: Event) => {
  e.preventDefault();
  await onSubmit(title(), desc());
  setTitle("");
  setDesc("");
};
---
<form onSubmit={handleSubmit} class="task-form">
  <input type="text" placeholder="New task" value={title()} onInput={(e) => setTitle(e.currentTarget.value)} required />
  <textarea placeholder="Description" value={desc()} onInput={(e) => setDesc(e.currentTarget.value)}></textarea>
  <button type="submit">Add</button>
</form>
```

#### `index.astro`

```astro
---
import { onMount, createSignal } from "solid-js";
import { getTasks, createTask, updateTask, deleteTask } from '../lib/api';
import TaskItem from '../components/TaskItem.astro';
import TaskForm from '../components/TaskForm.astro';
import type { Task } from '../lib/types';

const [tasks, setTasks] = createSignal<Task[]>([]);
const fetch = async () => setTasks(await getTasks());
onMount(fetch);

const add = async (title: string, desc?: string) => {
  const newTask = await createTask(title, desc);
  setTasks([...tasks(), newTask]);
};

const toggle = async (t: Task) => {
  const updated = await updateTask(t.id, { completed: !t.completed });
  setTasks(tasks().map(x => (x.id === updated.id ? updated : x)));
};

const remove = async (t: Task) => {
  await deleteTask(t.id);
  setTasks(tasks().filter(x => x.id !== t.id));
};
---
<html>
  <head><title>To‑Do List</title></head>
  <body>
    <h1>To‑Do List</h1>
    <TaskForm onSubmit={add} />
    <ul>{tasks().map(t => (<TaskItem task={t} onToggle={() => toggle(t)} onDelete={() => remove(t)} />))}</ul>
  </body>
</html>
```

## Development Workflow

1. **Install Rust** – `rustup toolchain install stable`
2. **Build & Run backend**
   ```bash
   cd server
   cargo run --release
   ```
3. **Install Node deps**
   ```bash
   cd web
   npm install
   ```
4. **Start frontend dev server**
   ```bash
   npm run dev
   ```
   *Astro will proxy `/api` to `http://localhost:3000` automatically.*
5. **Build for production**
   ```bash
   npm run build
   ```
6. **Deploy** – Optionally push the Astro build to Cloudflare Pages, Netlify, or serve with a static web server.

## Testing

- **Backend** – `cargo test` with an in‑memory SQLite DB.
- **Frontend** – `npm test` (Vitest + Playwright) for unit & E2E tests.

## Optional Docker Setup

Create a `Dockerfile` in the `server` folder for a lightweight image that contains the compiled binary and SQLite file. Build and push to a registry for container deployment.

---

This documentation can be copied into a new Codex session or GitHub repository to start implementation immediately.

