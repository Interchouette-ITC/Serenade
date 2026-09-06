//! In-memory mocks proving the framework contracts compile and behave.

use super::{PageRequest, PersistenceError, UnitOfWork};

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

#[tokio::test]
async fn unit_of_work_mock_runs_transaction_lifecycle() {
    let mut uow = MockUnitOfWork::default();
    UnitOfWork::begin(&mut uow).await.expect("begin");
    UnitOfWork::commit(&mut uow).await.expect("commit");
    let err = UnitOfWork::commit(&mut uow).await.expect_err("no tx");
    assert!(matches!(err, PersistenceError::InvalidInput { .. }));

    UnitOfWork::begin(&mut uow).await.expect("begin again");
    UnitOfWork::rollback(&mut uow).await.expect("rollback");
    let err = UnitOfWork::rollback(&mut uow).await.expect_err("no tx");
    assert!(matches!(err, PersistenceError::InvalidInput { .. }));
}

#[test]
fn page_request_first_sets_offset_zero() {
    let page = PageRequest::first(25);
    assert_eq!(page.limit, 25);
    assert_eq!(page.offset, 0);
}

#[test]
fn entity_id_string_and_version() {
    use super::EntityId;
    let id = String::from("row-1");
    assert_eq!(EntityId::as_str(&id), "row-1");
    assert_eq!(super::version(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn persist_param_policy_default_from_env() {
    use super::PersistParamPolicy;
    let _ = PersistParamPolicy::default();
}
