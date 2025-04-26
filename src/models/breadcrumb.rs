#[derive(serde::Serialize, Debug)]
pub struct Breadcrumb {
    pub id: Option<i32>,
    pub name: Option<String>,
}
