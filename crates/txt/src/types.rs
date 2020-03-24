use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub struct Section<'a> {
  pub name: &'a str,
  pub props: HashMap<&'a str, &'a str>,
}
