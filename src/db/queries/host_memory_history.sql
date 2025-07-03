SELECT
  memory_total,
  memory_used,
  memory_percentage,
  timestamp
FROM
  usage
WHERE
  container IS NULL
  AND timestamp BETWEEN :from
  AND :to
ORDER BY
  timestamp ASC;