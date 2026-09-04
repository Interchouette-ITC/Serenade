//! In-memory mock adapters proving the trait contracts compile and behave.

use super::{
    CartRepository, CategoryRepository, OrderRepository, PageRequest, PersistenceError,
    ProductRepository, UnitOfWork,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProductRow {
    id: String,
    slug: String,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CategoryRow {
    id: String,
    parent_id: Option<String>,
    slug: String,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CartRow {
    id: String,
    token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderRow {
    id: String,
    number: String,
    idempotency_key: Option<String>,
}

#[derive(Default)]
struct CatalogStore {
    products: HashMap<String, ProductRow>,
    categories: HashMap<String, CategoryRow>,
    carts: HashMap<String, CartRow>,
    orders: HashMap<String, OrderRow>,
    idempotency: HashMap<String, String>,
}

#[derive(Clone, Default)]
struct MockCatalog {
    store: Arc<Mutex<CatalogStore>>,
}

impl ProductRepository for MockCatalog {
    type Error = PersistenceError;
    type Id = String;
    type Product = ProductRow;

    async fn find_by_id(&self, id: &Self::Id) -> Result<Option<Self::Product>, Self::Error> {
        let store = self.store.lock().await;
        Ok(store.products.get(id).cloned())
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Self::Product>, Self::Error> {
        let store = self.store.lock().await;
        Ok(store
            .products
            .values()
            .find(|row| row.slug == slug)
            .cloned())
    }

    async fn list(&self, page: PageRequest) -> Result<Vec<Self::Product>, Self::Error> {
        let mut rows = {
            let store = self.store.lock().await;
            store.products.values().cloned().collect::<Vec<_>>()
        };
        rows.sort_by(|left, right| left.slug.cmp(&right.slug));
        let start = page.offset as usize;
        Ok(rows
            .into_iter()
            .skip(start)
            .take(page.limit as usize)
            .collect())
    }
}

impl CategoryRepository for MockCatalog {
    type Error = PersistenceError;
    type Id = String;
    type Category = CategoryRow;

    async fn find_by_id(&self, id: &Self::Id) -> Result<Option<Self::Category>, Self::Error> {
        let store = self.store.lock().await;
        Ok(store.categories.get(id).cloned())
    }

    async fn find_by_slug(
        &self,
        slug: &str,
        parent_id: Option<&Self::Id>,
    ) -> Result<Option<Self::Category>, Self::Error> {
        let store = self.store.lock().await;
        Ok(store
            .categories
            .values()
            .find(|row| {
                row.slug == slug && row.parent_id.as_deref() == parent_id.map(String::as_str)
            })
            .cloned())
    }

    async fn list_children(
        &self,
        parent_id: Option<&Self::Id>,
        page: PageRequest,
    ) -> Result<Vec<Self::Category>, Self::Error> {
        let mut rows = {
            let store = self.store.lock().await;
            store
                .categories
                .values()
                .filter(|row| row.parent_id.as_deref() == parent_id.map(String::as_str))
                .cloned()
                .collect::<Vec<_>>()
        };
        rows.sort_by(|left, right| left.slug.cmp(&right.slug));
        let start = page.offset as usize;
        Ok(rows
            .into_iter()
            .skip(start)
            .take(page.limit as usize)
            .collect())
    }
}

impl CartRepository for MockCatalog {
    type Error = PersistenceError;
    type Id = String;
    type Cart = CartRow;

    async fn find_by_token(&self, token: &str) -> Result<Option<Self::Cart>, Self::Error> {
        let store = self.store.lock().await;
        Ok(store.carts.values().find(|row| row.token == token).cloned())
    }

    async fn save(&self, cart: &Self::Cart) -> Result<(), Self::Error> {
        self.store
            .lock()
            .await
            .carts
            .insert(cart.id.clone(), cart.clone());
        Ok(())
    }

    async fn delete(&self, id: &Self::Id) -> Result<(), Self::Error> {
        self.store.lock().await.carts.remove(id);
        Ok(())
    }
}

impl OrderRepository for MockCatalog {
    type Error = PersistenceError;
    type Id = String;
    type Order = OrderRow;

    async fn find_by_number(&self, number: &str) -> Result<Option<Self::Order>, Self::Error> {
        let store = self.store.lock().await;
        Ok(store
            .orders
            .values()
            .find(|row| row.number == number)
            .cloned())
    }

    async fn save(&self, order: &Self::Order) -> Result<(), Self::Error> {
        self.store
            .lock()
            .await
            .orders
            .insert(order.id.clone(), order.clone());
        Ok(())
    }

    async fn save_idempotent(
        &self,
        order: &Self::Order,
        idempotency_key: &str,
    ) -> Result<(), Self::Error> {
        let mut store = self.store.lock().await;
        if let Some(existing_id) = store.idempotency.get(idempotency_key) {
            if existing_id != &order.id {
                return Err(PersistenceError::Conflict {
                    constraint: "idempotency_key",
                });
            }
        } else {
            store
                .idempotency
                .insert(idempotency_key.to_owned(), order.id.clone());
            store.orders.insert(order.id.clone(), order.clone());
        }
        drop(store);
        Ok(())
    }
}

#[derive(Default)]
struct MockUnitOfWork {
    active: bool,
}

impl UnitOfWork for MockUnitOfWork {
    type Error = PersistenceError;

    async fn begin(&mut self) -> Result<(), Self::Error> {
        tokio::task::yield_now().await;
        if self.active {
            return Err(PersistenceError::InvalidInput {
                message: "transaction already active".to_owned(),
            });
        }
        self.active = true;
        Ok(())
    }

    async fn commit(&mut self) -> Result<(), Self::Error> {
        tokio::task::yield_now().await;
        if !self.active {
            return Err(PersistenceError::InvalidInput {
                message: "no active transaction".to_owned(),
            });
        }
        self.active = false;
        Ok(())
    }

    async fn rollback(&mut self) -> Result<(), Self::Error> {
        tokio::task::yield_now().await;
        if !self.active {
            return Err(PersistenceError::InvalidInput {
                message: "no active transaction".to_owned(),
            });
        }
        self.active = false;
        Ok(())
    }
}

async fn seed_catalog() -> MockCatalog {
    let catalog = MockCatalog::default();
    let mut store = catalog.store.lock().await;
    store.products.insert(
        "p1".to_owned(),
        ProductRow {
            id: "p1".to_owned(),
            slug: "hoodie".to_owned(),
            name: "Hoodie".to_owned(),
        },
    );
    store.categories.insert(
        "c1".to_owned(),
        CategoryRow {
            id: "c1".to_owned(),
            parent_id: None,
            slug: "apparel".to_owned(),
            name: "Apparel".to_owned(),
        },
    );
    drop(store);
    catalog
}

#[tokio::test]
async fn product_repository_mock_lists_and_finds() {
    let catalog = seed_catalog().await;
    let by_slug = ProductRepository::find_by_slug(&catalog, "hoodie")
        .await
        .expect("find_by_slug")
        .expect("product");
    assert_eq!(by_slug.name, "Hoodie");

    let page = ProductRepository::list(&catalog, PageRequest::first(10))
        .await
        .expect("list");
    assert_eq!(page.len(), 1);
}

#[tokio::test]
async fn order_repository_mock_enforces_idempotency() {
    let catalog = seed_catalog().await;
    let order = OrderRow {
        id: "o1".to_owned(),
        number: "1001".to_owned(),
        idempotency_key: Some("key-1".to_owned()),
    };
    OrderRepository::save_idempotent(&catalog, &order, "key-1")
        .await
        .expect("first save");
    OrderRepository::save_idempotent(&catalog, &order, "key-1")
        .await
        .expect("replay");

    let conflict = OrderRow {
        id: "o2".to_owned(),
        number: "1002".to_owned(),
        idempotency_key: Some("key-1".to_owned()),
    };
    let err = OrderRepository::save_idempotent(&catalog, &conflict, "key-1")
        .await
        .expect_err("conflict");
    assert!(matches!(err, PersistenceError::Conflict { .. }));
}

#[tokio::test]
async fn unit_of_work_mock_runs_transaction_lifecycle() {
    let mut uow = MockUnitOfWork::default();
    UnitOfWork::begin(&mut uow).await.expect("begin");
    UnitOfWork::commit(&mut uow).await.expect("commit");
    let err = UnitOfWork::commit(&mut uow).await.expect_err("no tx");
    assert!(matches!(err, PersistenceError::InvalidInput { .. }));
}

#[test]
fn entity_id_string_and_version() {
    use super::EntityId;
    let id = String::from("sku-1");
    assert_eq!(EntityId::as_str(&id), "sku-1");
    assert_eq!(super::version(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn persist_param_policy_default_from_env() {
    use super::PersistParamPolicy;
    let _ = PersistParamPolicy::default();
}
