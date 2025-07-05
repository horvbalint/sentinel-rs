SELECT
  cpu_percentage AS percentage,
  timestamp
FROM
  usage
WHERE
  container IS NULL
  AND timestamp BETWEEN :from
  AND :to
ORDER BY
  timestamp ASC;