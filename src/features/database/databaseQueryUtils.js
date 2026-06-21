export const SQL_COMPLETION_KEYWORDS = [
  "SELECT", "FROM", "WHERE", "JOIN", "LEFT JOIN", "RIGHT JOIN", "INNER JOIN", "ORDER BY", "GROUP BY",
  "HAVING", "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE",
  "ALTER", "DROP", "TABLE", "VIEW", "INDEX", "PRIMARY KEY", "FOREIGN KEY", "DISTINCT", "COUNT",
  "SUM", "AVG", "MIN", "MAX", "AS", "AND", "OR", "NOT", "NULL", "IS", "IN", "LIKE", "BETWEEN",
  "EXISTS", "CASE", "WHEN", "THEN", "ELSE", "END", "UNION", "ALL", "DESC", "ASC",
];

export function quoteIdentifier(value) {
  return `\`${String(value).replaceAll("`", "``")}\``;
}

export function quoteString(value) {
  return `'${String(value).replaceAll("\\", "\\\\").replaceAll("'", "''")}'`;
}

export function defaultQuerySql(schema) {
  return schema ? `-- ${schema}\nSELECT 1;` : "SELECT 1;";
}
