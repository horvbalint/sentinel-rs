SELECT
  cpu_percentage,
  timestamp
FROM
  usage
WHERE
  container LIKE (:container || '%')
  AND timestamp BETWEEN :from
  AND :to
ORDER BY
  timestamp ASC;