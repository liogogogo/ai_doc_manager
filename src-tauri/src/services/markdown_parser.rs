use pulldown_cmark::{Event, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
pub struct MarkdownSection {
    pub heading: String,
    pub level: u32,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Parse a markdown file into logical sections by headings
pub fn parse_sections(text: &str) -> Vec<MarkdownSection> {
    let mut sections: Vec<MarkdownSection> = Vec::new();
    let mut current_heading = String::new();
    let mut current_level: u32 = 0;
    let mut current_content = String::new();
    let mut in_heading = false;
    let mut line_count = 0;
    let mut section_start = 0;

    let parser = Parser::new(text);

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // Save previous section
                if !current_heading.is_empty() || !current_content.is_empty() {
                    sections.push(MarkdownSection {
                        heading: current_heading.clone(),
                        level: current_level,
                        content: current_content.clone(),
                        start_line: section_start,
                        end_line: line_count,
                    });
                }
                current_heading.clear();
                current_content.clear();
                current_level = level as u32;
                section_start = line_count;
                in_heading = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
            }
            Event::Text(text) => {
                if in_heading {
                    current_heading.push_str(&text);
                } else {
                    current_content.push_str(&text);
                }
                line_count += text.matches('\n').count();
            }
            Event::SoftBreak | Event::HardBreak => {
                current_content.push('\n');
                line_count += 1;
            }
            _ => {}
        }
    }

    // Push final section
    if !current_heading.is_empty() || !current_content.is_empty() {
        sections.push(MarkdownSection {
            heading: current_heading,
            level: current_level,
            content: current_content,
            start_line: section_start,
            end_line: line_count,
        });
    }

    sections
}

/// Count lines in a file
pub fn count_lines(text: &str) -> usize {
    text.lines().count()
}

/// Split text into chunks of approximately `chunk_size` characters
pub fn chunk_text(text: &str, chunk_size: usize) -> Vec<(String, usize, usize)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut chunk_start = 0;

    for (i, line) in lines.iter().enumerate() {
        if current_chunk.len() + line.len() > chunk_size && !current_chunk.is_empty() {
            chunks.push((current_chunk.clone(), chunk_start, i));
            current_chunk.clear();
            chunk_start = i;
        }
        if !current_chunk.is_empty() {
            current_chunk.push('\n');
        }
        current_chunk.push_str(line);
    }

    if !current_chunk.is_empty() {
        chunks.push((current_chunk, chunk_start, lines.len()));
    }

    chunks
}
