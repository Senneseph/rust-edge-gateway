# Path Parameters

Extract dynamic values from URL paths.

## Basic Example

**Endpoint Path:** `/users/{id}`

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    // Extract the {id} parameter
    let user_id = match req.path_param("id") {
        Some(id) => id,
        None => return Response::bad_request("Missing user ID"),
    };
    
    Response::ok(json!({
        "user_id": user_id,
        "message": format!("Fetching user {}", user_id),
    }))
}

handler_loop!(handle);
```

## Test

```bash
curl http://localhost:9080/users/123
```

## Response

```json
{
  "user_id": "123",
  "message": "Fetching user 123"
}
```

## Multiple Parameters

**Endpoint Path:** `/users/{user_id}/posts/{post_id}`

```rust
fn handle(req: Request) -> Response {
    let user_id = req.path_param("user_id");
    let post_id = req.path_param("post_id");
    
    match (user_id, post_id) {
        (Some(uid), Some(pid)) => {
            Response::ok(json!({
                "user_id": uid,
                "post_id": pid,
            }))
        }
        _ => Response::bad_request("Missing parameters"),
    }
}
```

### Test

```bash
curl http://localhost:9080/users/42/posts/7
```

### Response

```json
{
  "user_id": "42",
  "post_id": "7"
}
```

## Type Conversion

Path parameters are always strings. Convert them to other types:

```rust
fn handle(req: Request) -> Response {
    let id_str = req.path_param("id")
        .ok_or("Missing ID")?;
    
    // Parse to integer
    let id: i64 = match id_str.parse() {
        Ok(n) => n,
        Err(_) => return Response::bad_request("ID must be a number"),
    };
    
    // Parse to UUID
    let uuid_str = req.path_param("uuid")
        .ok_or("Missing UUID")?;
    
    let uuid = match uuid::Uuid::parse_str(uuid_str) {
        Ok(u) => u,
        Err(_) => return Response::bad_request("Invalid UUID format"),
    };
    
    Response::ok(json!({
        "id": id,
        "uuid": uuid.to_string(),
    }))
}
```

## Optional Parameters with Defaults

```rust
fn handle(req: Request) -> Response {
    // Get page number, default to 1
    let page: u32 = req.path_param("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1);
    
    Response::ok(json!({"page": page}))
}
```

## Route Patterns

| Pattern | Matches | Parameters |
|---------|---------|------------|
| `/users/{id}` | `/users/123` | `id: "123"` |
| `/api/{version}/items` | `/api/v2/items` | `version: "v2"` |
| `/files/{path}` | `/files/docs` | `path: "docs"` |
| `/{org}/{repo}/issues/{num}` | `/acme/proj/issues/42` | `org: "acme"`, `repo: "proj"`, `num: "42"` |

## Common Patterns

### Resource by ID

```rust
// GET /users/{id}
fn get_user(req: Request) -> Response {
    let id = req.path_param("id").unwrap();
    // Fetch user by ID
    Response::ok(json!({"id": id, "name": "John"}))
}
```

### Nested Resources

```rust
// GET /organizations/{org_id}/teams/{team_id}/members
fn get_team_members(req: Request) -> Response {
    let org_id = req.path_param("org_id").unwrap();
    let team_id = req.path_param("team_id").unwrap();
    
    Response::ok(json!({
        "organization": org_id,
        "team": team_id,
        "members": ["alice", "bob"],
    }))
}
```

### Slug-based Routes

```rust
// GET /blog/{slug}
fn get_blog_post(req: Request) -> Response {
    let slug = req.path_param("slug").unwrap();
    
    // Lookup by slug
    Response::ok(json!({
        "slug": slug,
        "title": format!("Post: {}", slug),
    }))
}
```

