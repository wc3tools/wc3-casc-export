use crate::parse;
use crate::types::*;
use anyhow::anyhow;
use anyhow::Result;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::iter::{Enumerate, Iterator};
use std::path::Path;

pub struct SylkReader {
  source: Box<dyn BufRead>,
}

impl SylkReader {
  pub fn open_file<P: AsRef<Path>>(path: P) -> Result<Self> {
    Ok(SylkReader {
      source: Box::new(BufReader::new(File::open(path)?)),
    })
  }

  pub fn rows(self) -> SylkRows {
    SylkRows {
      row: None,
      lines: self.source.lines().enumerate(),
    }
  }

  pub fn read_all(self) -> Result<Vec<Row>> {
    use std::iter::FromIterator;
    Result::from_iter(self.rows())
  }
}

pub struct SylkRows {
  row: Option<Row>,
  lines: Enumerate<std::io::Lines<Box<dyn BufRead>>>,
}

impl Iterator for SylkRows {
  type Item = Result<Row>;

  fn next(&mut self) -> Option<Self::Item> {
    let mut row = self.row.take().unwrap_or_else(|| Row {
      row: 1,
      cells: vec![],
    });
    while let Some((line_idx, line)) = self.lines.next() {
      let line_number = line_idx + 1;
      match line {
        Ok(line) => match parse::parse_record(&line) {
          Ok(record) => {
            if let RecordType::C = record.ty {
              let x = record.fields.iter().find(|f| f.ty == FieldType::X);
              let y = record.fields.iter().find(|f| f.ty == FieldType::Y);

              let col = if let Some(x) = x {
                if let Value::Integer(v) = x.value {
                  v
                } else {
                  return Some(Err(anyhow!(
                    "line {}: invalid X value: {:?}",
                    line_number,
                    x.value
                  )));
                }
              } else {
                1
              };

              if let Some(y) = y {
                let row_number = if let Value::Integer(v) = y.value {
                  v
                } else {
                  return Some(Err(anyhow!(
                    "line {}: invalid Y value: {:?}",
                    line_number,
                    y.value
                  )));
                };
                if row_number as usize != row.row {
                  self.row = Some(Row {
                    row: row_number as usize,
                    cells: vec![Cell::from_record(col as usize, record)],
                  });
                  return Some(Ok(row));
                }
              }

              if col == 1 && !row.cells.is_empty() {
                return Some(Err(anyhow!("line {}: missing X field", line_number)));
              }

              row.cells.push(Cell::from_record(col as usize, record));
            }
          }
          Err(e) => return Some(Err(e)),
        },
        Err(e) => return Some(Err(e.into())),
      }
    }
    if !row.cells.is_empty() {
      Some(Ok(row))
    } else {
      None
    }
  }
}

#[derive(Debug)]
pub struct Row {
  pub row: usize,
  pub cells: Vec<Cell>,
}

#[derive(Debug)]

pub struct Cell {
  /// X: column position (one based)
  pub column: usize,
  /// K: value of the cell
  pub value: Option<CellValue>,
}

impl Cell {
  fn from_record(column: usize, record: Record) -> Self {
    Cell {
      column,
      value: match record.fields.iter().find(|f| f.ty == FieldType::K) {
        Some(f) => match f.value {
          Value::Unknown(_) => None,
          Value::Integer(i) => Some(CellValue::Number(i as f64)),
          Value::Number(f) => Some(CellValue::Number(f)),
          Value::Text(ref t) => Some(CellValue::Text(t.to_string())),
        },
        None => None,
      },
    }
  }
}

#[derive(Debug)]
pub enum CellValue {
  Text(String),
  Number(f64),
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_read_file() {
    let r = SylkReader::open_file("sample/unitabilities.slk").unwrap();
    let rows = r.read_all().unwrap();
    assert_eq!(rows.len(), 865);

    let r = SylkReader::open_file("sample/unitdata.slk").unwrap();
    let rows = r.read_all().unwrap();
    assert_eq!(rows.len(), 865);
  }
}
