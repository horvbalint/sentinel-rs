SELECT
  memory_total,
  memory_used,
  memory_percentage,
  timestamp
FROM
  usage
WHERE
  container LIKE (:container || '%')
  AND timestamp BETWEEN :from
  AND :to
ORDER BY
  timestamp ASC