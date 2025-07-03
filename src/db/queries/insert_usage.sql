INSERT INTO
  usage (
    timestamp,
    container,
    cpu_percentage,
    memory_total,
    memory_used,
    memory_percentage
  )
VALUES
  (
    :timestamp,
    :container,
    :cpu_percentage,
    :memory_total,
    :memory_used,
    :memory_percentage
  );