//! Minimal application built only on `serenade-framework`.

use std::sync::{Arc, Mutex};

use serenade::event::EventError;
use serenade::http::{Method, Request, Response};
use serenade::prelude::*;
use serenade::validator::{Constraint, ConstraintViolationList, NotBlank};

type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

const CONFIG: &str = "[shop]\nname = \"Serenade Shop\"\n";

struct ShopBundle;

impl BundleInterface for ShopBundle {
    fn name(&self) -> &'static str {
        "shop"
    }
}

struct ShopReady;

impl Event for ShopReady {
    fn name(&self) -> &'static str {
        "shop.ready"
    }
}

struct Journal(Arc<Mutex<Vec<String>>>);

impl EventSubscriber for Journal {
    fn event_name(&self) -> &'static str {
        "shop.ready"
    }

    fn handle(&self, event: &dyn Event) -> Result<(), EventError> {
        if let Ok(mut journal) = self.0.lock() {
            journal.push(event.name().to_owned());
        }
        Ok(())
    }
}

fn handle(request: &Request, shop_name: &str) -> Response {
    let mut violations = ConstraintViolationList::new();
    NotBlank.validate(shop_name, "shop.name", &mut violations);
    if !violations.is_empty() {
        return Response::text(500, "invalid configuration");
    }
    match (request.method(), request.path()) {
        (Method::Get, "/") => Response::text(200, format!("Welcome to {shop_name}")),
        _ => Response::text(404, "not found"),
    }
}

/// Boots the app, handles one request, and returns status, body, and dispatched events.
fn run(path: &str) -> AppResult<(u16, String, Vec<String>)> {
    let mut app = App::new(Environment::Dev);
    app.register_bundle(ShopBundle)?;
    app.boot()?;

    let config = Config::from_toml(CONFIG)?;
    let mut builder = ContainerBuilder::new();
    config.apply_to(builder.parameters_mut());
    let container = builder.compile()?;
    let shop_name = container.parameters().get("shop.name")?.to_owned();

    let journal = Arc::new(Mutex::new(Vec::new()));
    let mut dispatcher = EventDispatcher::new();
    dispatcher.add(Arc::new(Journal(Arc::clone(&journal))));
    dispatcher.dispatch(&ShopReady)?;

    let response = handle(&Request::new(Method::Get, path), &shop_name);
    app.shutdown()?;

    let events = journal
        .lock()
        .map_or_else(|_| Vec::new(), |events| events.clone());
    Ok((
        response.status(),
        String::from_utf8_lossy(response.body()).into_owned(),
        events,
    ))
}

fn main() -> AppResult<()> {
    let (status, body, events) = run("/")?;
    println!("serenade-framework {}", serenade::version());
    println!("{status} {body}");
    println!("events: {}", events.join(", "));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn home_page_uses_configured_shop_name() {
        let (status, body, events) = run("/").expect("run");
        assert_eq!(status, 200);
        assert_eq!(body, "Welcome to Serenade Shop");
        assert_eq!(events, vec!["shop.ready".to_owned()]);
    }

    #[test]
    fn unknown_path_is_not_found() {
        let (status, _, _) = run("/missing").expect("run");
        assert_eq!(status, 404);
    }
}
