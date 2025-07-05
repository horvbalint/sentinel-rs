SELECT
  memory_total AS total,
  memory_used AS used,
  memory_percentage AS percentage,
  timestamp
FROM
  usage
WHERE
  container IS NULL
ORDER BY
  timestamp DESC
LIMIT
  1;