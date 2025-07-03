SELECT
  memory_total,
  memory_used,
  memory_percentage,
  timestamp
FROM
  usage
WHERE
  container IS NULL
ORDER BY
  timestamp DESC
LIMIT
  1;