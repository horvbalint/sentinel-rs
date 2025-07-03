SELECT
  cpu_percentage,
  timestamp
FROM
  usage
WHERE
  container IS NULL
ORDER BY
  timestamp DESC
LIMIT
  1;