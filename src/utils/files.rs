use tokio::io::AsyncBufReadExt;

pub async fn read_file_lines(path: &str) -> eyre::Result<Vec<String>> {
    let file = tokio::fs::read(path).await?;
    let mut lines = file.lines();

    let mut lines_vec = vec![];
    while let Some(line) = lines.next_line().await? {
        lines_vec.push(line)
    }

    Ok(lines_vec)
}
