SELECT
  cpu_percentage AS percentage,
  timestamp
FROM
  usage
WHERE
  container IS NULL
ORDER BY
  timestamp DESC
LIMIT
  1;