SELECT
  memory_total AS total,
  memory_used AS used,
  memory_percentage AS percentage,
  timestamp
FROM
  usage
WHERE
  container LIKE (:container || '%')
  AND timestamp BETWEEN :from
  AND :to
ORDER BY
  timestamp ASC