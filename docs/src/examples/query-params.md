# Query Parameters

Access URL query string values.

## Basic Example

**Endpoint Path:** `/search`

```rust
use rust_edge_gateway_sdk::prelude::*;

fn handle(req: Request) -> Response {
    // Get query parameter
    let query = req.query_param("q")
        .map(|s| s.to_string())
        .unwrap_or_default();
    
    if query.is_empty() {
        return Response::bad_request("Missing search query");
    }
    
    Response::ok(json!({
        "query": query,
        "results": [],
    }))
}

handler_loop!(handle);
```

## Test

```bash
curl "http://localhost:9080/search?q=rust"
```

## Response

```json
{
  "query": "rust",
  "results": []
}
```

## Pagination Example

```rust
fn handle(req: Request) -> Response {
    // Parse pagination parameters with defaults
    let page: u32 = req.query_param("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1);
    
    let limit: u32 = req.query_param("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(10)
        .min(100);  // Cap at 100
    
    let offset = (page - 1) * limit;
    
    Response::ok(json!({
        "page": page,
        "limit": limit,
        "offset": offset,
        "items": [],
    }))
}
```

### Test

```bash
curl "http://localhost:9080/items?page=2&limit=20"
```

### Response

```json
{
  "page": 2,
  "limit": 20,
  "offset": 20,
  "items": []
}
```

## Filtering Example

```rust
fn handle(req: Request) -> Response {
    // Get filter parameters
    let status = req.query_param("status");
    let category = req.query_param("category");
    let min_price: Option<f64> = req.query_param("min_price")
        .and_then(|p| p.parse().ok());
    let max_price: Option<f64> = req.query_param("max_price")
        .and_then(|p| p.parse().ok());
    
    Response::ok(json!({
        "filters": {
            "status": status,
            "category": category,
            "price_range": {
                "min": min_price,
                "max": max_price,
            },
        },
        "items": [],
    }))
}
```

### Test

```bash
curl "http://localhost:9080/products?status=active&category=electronics&min_price=100"
```

## Sorting Example

```rust
fn handle(req: Request) -> Response {
    let sort_by = req.query_param("sort")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "created_at".to_string());
    
    let order = req.query_param("order")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "desc".to_string());
    
    // Validate sort field
    let valid_fields = ["name", "created_at", "price", "popularity"];
    if !valid_fields.contains(&sort_by.as_str()) {
        return Response::bad_request(format!(
            "Invalid sort field. Valid options: {:?}", valid_fields
        ));
    }
    
    Response::ok(json!({
        "sort": {
            "field": sort_by,
            "order": order,
        },
        "items": [],
    }))
}
```

## Boolean Parameters

```rust
fn handle(req: Request) -> Response {
    // Parse boolean parameters
    let include_deleted = req.query_param("include_deleted")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    
    let verbose = req.query_param("verbose")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    
    Response::ok(json!({
        "include_deleted": include_deleted,
        "verbose": verbose,
    }))
}
```

## All Query Parameters

Access all query parameters at once:

```rust
fn handle(req: Request) -> Response {
    // Log all query parameters
    for (key, value) in &req.query {
        eprintln!("Query param: {} = {}", key, value);
    }
    
    Response::ok(json!({
        "query_params": req.query,
    }))
}
```

## Validation Helper

Create a reusable validation function:

```rust
fn parse_pagination(req: &Request) -> Result<(u32, u32), Response> {
    let page: u32 = req.query_param("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1);
    
    if page == 0 {
        return Err(Response::bad_request("Page must be >= 1"));
    }
    
    let limit: u32 = req.query_param("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(10);
    
    if limit > 100 {
        return Err(Response::bad_request("Limit must be <= 100"));
    }
    
    Ok((page, limit))
}

fn handle(req: Request) -> Response {
    let (page, limit) = match parse_pagination(&req) {
        Ok(p) => p,
        Err(response) => return response,
    };
    
    Response::ok(json!({"page": page, "limit": limit}))
}
```

