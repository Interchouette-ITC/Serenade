use leptos::prelude::{Get, RwSignal, Set, Update};
use rangular_host::{Host, HostError, Value};

pub struct CmsHelloHost {
    route_specific: RwSignal<bool>,
    clicks: RwSignal<u32>,
    navigate: Option<Box<dyn Fn(bool)>>,
}

impl CmsHelloHost {
    pub fn new(route_specific: bool) -> Self {
        Self {
            route_specific: RwSignal::new(route_specific),
            clicks: RwSignal::new(0),
            navigate: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_navigator(route_specific: bool, navigate: impl Fn(bool) + 'static) -> Self {
        Self {
            navigate: Some(Box::new(navigate)),
            ..Self::new(route_specific)
        }
    }

    #[allow(dead_code)]
    pub fn route_signal(&self) -> RwSignal<bool> {
        self.route_specific
    }
}

impl Host for CmsHelloHost {
    fn get(&self, name: &str) -> Option<Value> {
        let route_specific = self.route_specific.get();
        match name {
            "route_specific" => Some(Value::Bool(route_specific)),
            "route_title" => Some(Value::Str("Serenade CMS & <Hello>".to_owned())),
            "route_description" => Some(Value::Str(
                "A request-time generated description with & <angle brackets>.".to_owned(),
            )),
            "route_canonical" => Some(Value::Str(
                "/cms/hello?source=universal&mode=ssr".to_owned(),
            )),
            "route_social_image" => Some(Value::Str(
                "/serenade-banner.png?crop=cms&width=1200".to_owned(),
            )),
            "json_ld" => Some(Value::Str(
                r#"{"@context":"https://schema.org","name":"Serenade & <Universal>"}"#.to_owned(),
            )),
            "heading" => Some(Value::Str(if route_specific {
                "Hello from Rangular".to_owned()
            } else {
                "The shared Universal route".to_owned()
            })),
            "message" => Some(Value::Str(if route_specific {
                "This dynamic message contains & and <angle brackets>.".to_owned()
            } else {
                "This shared route is rendered in place without a full-page load.".to_owned()
            })),
            "click_label" => Some(Value::Str(format!("Clicked {} times", self.clicks.get()))),
            _ => None,
        }
    }

    fn call(&mut self, name: &str, _: &[Value]) -> Result<Value, HostError> {
        match name {
            "increment_clicks" => self.clicks.update(|clicks| *clicks += 1),
            "show_shared" => self.set_route(false),
            "show_route" => self.set_route(true),
            _ => return Err(HostError::UnknownMember(name.to_owned())),
        }
        Ok(Value::Unit)
    }
}

impl CmsHelloHost {
    fn set_route(&self, route_specific: bool) {
        if let Some(navigate) = &self.navigate {
            navigate(route_specific);
        }
        self.route_specific.set(route_specific);
    }
}

pub fn route_specific_from_query(query: Option<&str>) -> bool {
    !query
        .unwrap_or_default()
        .trim_start_matches('?')
        .split('&')
        .any(|parameter| parameter == "view=shared")
}
