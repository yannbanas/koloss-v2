use crate::synthesis::dsl::Grid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcTask {
    pub id: String,
    pub train: Vec<ArcExample>,
    pub test: Vec<ArcExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcExample {
    pub input: Grid,
    pub output: Grid,
}

pub fn load_arc_tasks(path: &str) -> anyhow::Result<Vec<ArcTask>> {
    let content = std::fs::read_to_string(path)?;
    let tasks: Vec<ArcTask> = serde_json::from_str(&content)?;
    Ok(tasks)
}

pub fn load_arc_task(path: &str) -> anyhow::Result<ArcTask> {
    let content = std::fs::read_to_string(path)?;
    let raw: serde_json::Value = serde_json::from_str(&content)?;

    let mut train = Vec::new();
    if let Some(train_arr) = raw.get("train").and_then(|v| v.as_array()) {
        for ex in train_arr {
            if let (Some(input), Some(output)) = (ex.get("input"), ex.get("output")) {
                train.push(ArcExample {
                    input: parse_grid(input),
                    output: parse_grid(output),
                });
            }
        }
    }

    let mut test = Vec::new();
    if let Some(test_arr) = raw.get("test").and_then(|v| v.as_array()) {
        for ex in test_arr {
            if let (Some(input), Some(output)) = (ex.get("input"), ex.get("output")) {
                test.push(ArcExample {
                    input: parse_grid(input),
                    output: parse_grid(output),
                });
            }
        }
    }

    let id = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(ArcTask { id, train, test })
}

fn parse_grid(val: &serde_json::Value) -> Grid {
    val.as_array()
        .map(|rows| {
            rows.iter().map(|row| {
                row.as_array()
                    .map(|cells| cells.iter().map(|c| c.as_u64().unwrap_or(0) as u8).collect())
                    .unwrap_or_default()
            }).collect()
        })
        .unwrap_or_default()
}

pub fn grid_to_string(grid: &Grid) -> String {
    grid.iter()
        .map(|row: &Vec<u8>| row.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn grid_dimensions(grid: &Grid) -> (usize, usize) {
    if grid.is_empty() { return (0, 0); }
    (grid.len(), grid[0].len())
}

pub fn unique_colors(grid: &Grid) -> Vec<u8> {
    let mut colors = Vec::new();
    for row in grid {
        for &c in row {
            if !colors.contains(&c) {
                colors.push(c);
            }
        }
    }
    colors.sort();
    colors
}
