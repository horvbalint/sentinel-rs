SELECT
  cpu_percentage AS percentage,
  timestamp
FROM
  usage
WHERE
  container LIKE (:container || '%')
ORDER BY
  timestamp DESC
LIMIT
  1;