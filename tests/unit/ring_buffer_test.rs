use shellwright::output::ring_buffer::RingBuffer;

#[test]
fn test_append_and_read() {
    let mut buf = RingBuffer::new(100);
    buf.append("line 1".to_string());
    buf.append("line 2".to_string());

    assert_eq!(buf.len(), 2);
    assert_eq!(buf.total_lines(), 2);
}

#[test]
fn test_cursor_based_reads() {
    let mut buf = RingBuffer::new(100);
    buf.append("line 1".to_string());
    buf.append("line 2".to_string());

    let (lines, cursor) = buf.read_since(0);
    assert_eq!(lines.len(), 2);
    assert_eq!(cursor, 2);

    buf.append("line 3".to_string());

    let (lines, cursor) = buf.read_since(cursor);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].line, "line 3");
    assert_eq!(cursor, 3);
}

#[test]
fn test_capacity_eviction() {
    let mut buf = RingBuffer::new(3);
    buf.append("a".to_string());
    buf.append("b".to_string());
    buf.append("c".to_string());
    buf.append("d".to_string());

    assert_eq!(buf.len(), 3);
    assert_eq!(buf.total_lines(), 4);

    // "a" should have been evicted
    let (lines, _) = buf.read_since(0);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].line, "b");
    assert_eq!(lines[2].line, "d");
}

#[test]
fn test_tail() {
    let mut buf = RingBuffer::new(100);
    for i in 0..10 {
        buf.append(format!("line {}", i));
    }

    let tail = buf.tail(3);
    assert_eq!(tail.len(), 3);
    assert_eq!(tail[0].line, "line 7");
    assert_eq!(tail[2].line, "line 9");
}

#[test]
fn test_tail_text() {
    let mut buf = RingBuffer::new(100);
    buf.append("hello".to_string());
    buf.append("world".to_string());

    let text = buf.tail_text(2);
    assert_eq!(text, "hello\nworld");
}

#[test]
fn test_contents() {
    let mut buf = RingBuffer::new(100);
    buf.append("a".to_string());
    buf.append("b".to_string());
    buf.append("c".to_string());

    assert_eq!(buf.contents(), "a\nb\nc");
}

#[test]
fn test_empty_buffer() {
    let buf = RingBuffer::new(100);
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
    assert_eq!(buf.cursor(), 0);

    let (lines, cursor) = buf.read_since(0);
    assert!(lines.is_empty());
    assert_eq!(cursor, 0);
}

#[test]
fn test_append_lines() {
    let mut buf = RingBuffer::new(100);
    buf.append_lines("line 1\nline 2\nline 3");

    assert_eq!(buf.len(), 3);
    let (lines, _) = buf.read_since(0);
    assert_eq!(lines[0].line, "line 1");
    assert_eq!(lines[2].line, "line 3");
}

#[test]
fn test_wrap_around_cursor_consistency() {
    let mut buf = RingBuffer::new(2);
    buf.append("a".to_string()); // index 0
    buf.append("b".to_string()); // index 1
    buf.append("c".to_string()); // index 2, evicts "a"

    // Reading since index 1 should give "b" and "c"
    let (lines, cursor) = buf.read_since(1);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].line, "b");
    assert_eq!(lines[1].line, "c");
    assert_eq!(cursor, 3);
}
