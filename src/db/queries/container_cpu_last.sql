SELECT
  cpu_percentage,
  timestamp
FROM
  usage
WHERE
  container LIKE (:container || '%')
ORDER BY
  timestamp DESC
LIMIT
  1;