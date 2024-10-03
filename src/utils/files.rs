use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncWriteExt},
};

pub async fn read_file_lines(path: &str) -> eyre::Result<Vec<String>> {
    let file = tokio::fs::read(path).await?;
    let mut lines = file.lines();

    let mut lines_vec = vec![];
    while let Some(line) = lines.next_line().await? {
        lines_vec.push(line)
    }

    Ok(lines_vec)
}

pub async fn append_line_to_file(file_path: &str, line: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .await?;

    file.write_all(format!("{}\n", line).as_bytes()).await?;
    Ok(())
}
