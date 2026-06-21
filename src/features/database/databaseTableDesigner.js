import { quoteIdentifier, quoteString } from "./databaseQueryUtils";

export function defaultDesignColumn() {
  return {
    name: "",
    typeName: "VARCHAR",
    length: "255",
    scale: "",
    nullable: true,
    virtual: false,
    key: "",
    defaultValue: "",
    primary: false,
    autoIncrement: false,
    comment: "",
    originalName: "",
    original: null,
    dropped: false,
  };
}

export function createDesignState(tab) {
  return {
    mode: tab.mode,
    loaded: tab.mode === "create",
    saving: false,
    originalTable: tab.table ?? "",
    tableName: tab.table ?? "",
    engine: "InnoDB",
    charset: "utf8mb4",
    collation: "utf8mb4_unicode_ci",
    tableComment: "",
    columns: tab.mode === "create"
      ? [{
          ...defaultDesignColumn(),
          name: "id",
          typeName: "BIGINT UNSIGNED",
          length: "",
          nullable: false,
          key: "PRIMARY",
          primary: true,
          autoIncrement: true,
        }]
      : [],
    indexes: [],
    foreignKeys: [],
    triggers: [],
    checks: [],
  };
}

function parseColumnType(columnType) {
  const text = String(columnType ?? "").trim();
  const match = text.match(/^(.+?)(?:\(([^)]*)\))?$/);
  const typeName = (match?.[1] ?? text).trim().toUpperCase();
  const [length = "", scale = ""] = String(match?.[2] ?? "").split(",").map((item) => item.trim());
  return { typeName, length, scale };
}

export function columnsFromTableDetail(detail) {
  const primaryColumns = new Set((detail.indexes ?? [])
    .filter((index) => index.name === "PRIMARY")
    .sort((left, right) => Number(left.seqInIndex) - Number(right.seqInIndex))
    .map((index) => index.columnName));

  return (detail.columns ?? []).map((column) => {
    const parsedType = parseColumnType(column.columnType);
    const normalized = {
      name: column.name,
      ...parsedType,
      nullable: column.nullable === "YES",
      virtual: false,
      key: primaryColumns.has(column.name) ? "PRIMARY" : column.key === "UNI" ? "UNIQUE" : column.key === "MUL" ? "INDEX" : "",
      defaultValue: column.defaultValue ?? "",
      primary: primaryColumns.has(column.name),
      autoIncrement: String(column.extra ?? "").toLowerCase().includes("auto_increment"),
      comment: column.comment ?? "",
    };

    return {
      ...normalized,
      originalName: column.name,
      original: { ...normalized },
      dropped: false,
    };
  });
}

export function indexesFromTableDetail(detail) {
  const grouped = new Map();
  for (const index of detail.indexes ?? []) {
    if (index.name === "PRIMARY") {
      continue;
    }
    const current = grouped.get(index.name) ?? {
      name: index.name,
      unique: Number(index.nonUnique) === 0,
      columns: [],
      type: index.indexType ?? "BTREE",
      originalName: index.name,
      original: null,
      dropped: false,
    };
    current.columns.push({
      name: index.columnName,
      seq: Number(index.seqInIndex),
    });
    grouped.set(index.name, current);
  }

  return [...grouped.values()].map((index) => {
    const normalized = {
      name: index.name,
      unique: index.unique,
      columns: index.columns.sort((left, right) => left.seq - right.seq).map((column) => column.name).join(", "),
      type: index.type,
    };
    return {
      ...normalized,
      originalName: index.name,
      original: { ...normalized },
      dropped: false,
    };
  });
}

export function foreignKeysFromTableDetail(detail) {
  const grouped = new Map();
  for (const key of detail.foreignKeys ?? []) {
    const current = grouped.get(key.name) ?? {
      name: key.name,
      columns: [],
      referencedTable: key.referencedTable,
      referencedColumns: [],
      onUpdate: key.updateRule,
      onDelete: key.deleteRule,
      originalName: key.name,
      original: null,
      dropped: false,
    };
    current.columns.push(key.columnName);
    current.referencedColumns.push(key.referencedColumn);
    grouped.set(key.name, current);
  }

  return [...grouped.values()].map((key) => {
    const normalized = {
      name: key.name,
      columns: key.columns.join(", "),
      referencedTable: key.referencedTable,
      referencedColumns: key.referencedColumns.join(", "),
      onUpdate: key.onUpdate,
      onDelete: key.onDelete,
    };
    return { ...normalized, originalName: key.name, original: { ...normalized }, dropped: false };
  });
}

export function triggersFromTableDetail(detail) {
  return (detail.triggers ?? []).map((trigger) => {
    const normalized = {
      name: trigger.name,
      timing: trigger.timing,
      event: trigger.event,
      statement: trigger.statement,
    };
    return { ...normalized, originalName: trigger.name, original: { ...normalized }, dropped: false };
  });
}

export function checksFromTableDetail(detail) {
  return (detail.checks ?? []).map((check) => {
    const normalized = {
      name: check.name,
      expression: check.expression,
      enforced: check.enforced !== "NO",
    };
    return { ...normalized, originalName: check.name, original: { ...normalized }, dropped: false };
  });
}

export function applyOptionsFromTableDetail(state, detail) {
  const options = detail.options ?? {};
  state.engine = options.engine || state.engine;
  state.collation = options.collation || state.collation;
  state.charset = options.collation?.split("_")?.[0] || state.charset;
  state.tableComment = options.comment ?? state.tableComment;
  state.originalOptions = {
    engine: state.engine,
    charset: state.charset,
    collation: state.collation,
    tableComment: state.tableComment,
  };
}

export function markDesignStateSaved(state) {
  state.mode = "edit";
  state.originalTable = state.tableName;
  for (const column of state.columns.filter((item) => !item.dropped)) {
    column.originalName = column.name;
    column.original = {
      name: column.name,
      typeName: column.typeName,
      length: column.length,
      scale: column.scale,
      nullable: column.nullable,
      virtual: column.virtual,
      key: column.key,
      defaultValue: column.defaultValue,
      primary: column.primary,
      autoIncrement: column.autoIncrement,
      comment: column.comment,
    };
    column.dropped = false;
  }
  state.columns = state.columns.filter((item) => !item.dropped);
  for (const index of state.indexes.filter((item) => !item.dropped)) {
    index.originalName = index.name;
    index.original = {
      name: index.name,
      unique: index.unique,
      columns: index.columns,
      type: index.type,
    };
    index.dropped = false;
  }
  state.indexes = state.indexes.filter((item) => !item.dropped);
  for (const foreignKey of state.foreignKeys.filter((item) => !item.dropped)) {
    foreignKey.originalName = foreignKey.name;
    foreignKey.original = {
      name: foreignKey.name,
      columns: foreignKey.columns,
      referencedTable: foreignKey.referencedTable,
      referencedColumns: foreignKey.referencedColumns,
      onUpdate: foreignKey.onUpdate,
      onDelete: foreignKey.onDelete,
    };
    foreignKey.dropped = false;
  }
  state.foreignKeys = state.foreignKeys.filter((item) => !item.dropped);
  for (const trigger of state.triggers.filter((item) => !item.dropped)) {
    trigger.originalName = trigger.name;
    trigger.original = {
      name: trigger.name,
      timing: trigger.timing,
      event: trigger.event,
      statement: trigger.statement,
    };
    trigger.dropped = false;
  }
  state.triggers = state.triggers.filter((item) => !item.dropped);
  for (const check of state.checks.filter((item) => !item.dropped)) {
    check.originalName = check.name;
    check.original = {
      name: check.name,
      expression: check.expression,
      enforced: check.enforced,
    };
    check.dropped = false;
  }
  state.checks = state.checks.filter((item) => !item.dropped);
  state.originalOptions = {
    engine: state.engine,
    charset: state.charset,
    collation: state.collation,
    tableComment: state.tableComment,
  };
}

export const MYSQL_COLUMN_TYPE_OPTIONS = [
  "BIGINT UNSIGNED",
  "BIGINT",
  "INT UNSIGNED",
  "INT",
  "TINYINT",
  "DECIMAL",
  "VARCHAR",
  "CHAR",
  "TEXT",
  "MEDIUMTEXT",
  "LONGTEXT",
  "JSON",
  "DATETIME",
  "TIMESTAMP",
  "DATE",
  "TIME",
  "BOOLEAN",
  "DOUBLE",
  "FLOAT",
  "BLOB",
  "LONGBLOB",
];

export const TABLE_DESIGN_SECTIONS = [
  { key: "fields", label: "字段" },
  { key: "indexes", label: "索引" },
  { key: "foreignKeys", label: "外键" },
  { key: "triggers", label: "触发器" },
  { key: "checks", label: "检查" },
  { key: "options", label: "选项" },
  { key: "comment", label: "注释" },
  { key: "sql", label: "SQL 预览" },
];

export function defaultDesignIndex() {
  return {
    name: "",
    unique: false,
    columns: "",
    type: "BTREE",
    originalName: "",
    original: null,
    dropped: false,
  };
}

export function defaultDesignForeignKey() {
  return {
    name: "",
    columns: "",
    referencedTable: "",
    referencedColumns: "",
    onUpdate: "RESTRICT",
    onDelete: "RESTRICT",
    originalName: "",
    original: null,
    dropped: false,
  };
}

export function defaultDesignTrigger() {
  return {
    name: "",
    timing: "BEFORE",
    event: "INSERT",
    statement: "",
    originalName: "",
    original: null,
    dropped: false,
  };
}

export function defaultDesignCheck() {
  return {
    name: "",
    expression: "",
    enforced: true,
    originalName: "",
    original: null,
    dropped: false,
  };
}

export function buildTableDesignSql(tab, state) {
  return state.mode === "create" ? buildCreateTableSql(tab, state) : buildAlterTableSql(tab, state);
}

function normalizeSqlIdentifier(value) {
  return String(value ?? "").trim();
}

function defaultClause(value) {
  const trimmed = String(value ?? "").trim();
  if (!trimmed) {
    return "";
  }
  if (/^(NULL|CURRENT_TIMESTAMP(?:\(\d+\))?|CURRENT_DATE|CURRENT_TIME)$/i.test(trimmed)) {
    return ` DEFAULT ${trimmed}`;
  }
  if (/^-?\d+(?:\.\d+)?$/.test(trimmed)) {
    return ` DEFAULT ${trimmed}`;
  }
  return ` DEFAULT ${quoteString(trimmed)}`;
}

function columnDefinition(column) {
  const name = normalizeSqlIdentifier(column.name);
  const parsedType = parseColumnType(column.typeName);
  const typeName = parsedType.typeName;
  const length = String(column.length || parsedType.length || "").trim();
  const scale = String(column.scale || parsedType.scale || "").trim();
  const type = length ? `${typeName}(${scale ? `${length},${scale}` : length})` : typeName;
  if (!name || !type) {
    throw new Error("字段名称和类型不能为空");
  }

  return [
    quoteIdentifier(name),
    type,
    column.nullable ? "NULL" : "NOT NULL",
    defaultClause(column.defaultValue),
    column.autoIncrement ? "AUTO_INCREMENT" : "",
    column.comment ? `COMMENT ${quoteString(column.comment)}` : "",
  ].filter(Boolean).join(" ");
}

function indexColumns(value) {
  return String(value ?? "")
    .split(",")
    .map((column) => normalizeSqlIdentifier(column))
    .filter(Boolean);
}

function indexDefinition(index) {
  const name = normalizeSqlIdentifier(index.name);
  const columns = indexColumns(index.columns);
  if (!name || columns.length === 0) {
    throw new Error("索引名称和字段不能为空");
  }
  const prefix = index.unique ? "UNIQUE KEY" : "KEY";
  const type = String(index.type ?? "").trim();
  const using = type ? ` USING ${type}` : "";
  return `${prefix} ${quoteIdentifier(name)} (${columns.map(quoteIdentifier).join(", ")})${using}`;
}

function foreignKeyDefinition(foreignKey) {
  const name = normalizeSqlIdentifier(foreignKey.name);
  const columns = indexColumns(foreignKey.columns);
  const referencedTable = normalizeSqlIdentifier(foreignKey.referencedTable);
  const referencedColumns = indexColumns(foreignKey.referencedColumns);
  if (!name || columns.length === 0 || !referencedTable || referencedColumns.length === 0) {
    throw new Error("外键名称、字段、引用表和引用字段不能为空");
  }
  return [
    `CONSTRAINT ${quoteIdentifier(name)}`,
    `FOREIGN KEY (${columns.map(quoteIdentifier).join(", ")})`,
    `REFERENCES ${quoteIdentifier(referencedTable)} (${referencedColumns.map(quoteIdentifier).join(", ")})`,
    foreignKey.onDelete ? `ON DELETE ${foreignKey.onDelete}` : "",
    foreignKey.onUpdate ? `ON UPDATE ${foreignKey.onUpdate}` : "",
  ].filter(Boolean).join(" ");
}

function checkDefinition(check) {
  const name = normalizeSqlIdentifier(check.name);
  const expression = String(check.expression ?? "").trim();
  if (!name || !expression) {
    throw new Error("检查名称和表达式不能为空");
  }
  return `CONSTRAINT ${quoteIdentifier(name)} CHECK (${expression})${check.enforced ? " ENFORCED" : " NOT ENFORCED"}`;
}

function triggerStatement(tab, trigger) {
  const name = normalizeSqlIdentifier(trigger.name);
  const statement = String(trigger.statement ?? "").trim();
  const tableName = normalizeSqlIdentifier(tab.tableName || tab.table);
  if (!name || !statement) {
    throw new Error("触发器名称和语句不能为空");
  }
  if (!tableName) {
    throw new Error("触发器所属表不能为空");
  }
  return `CREATE TRIGGER ${quoteIdentifier(tab.schema)}.${quoteIdentifier(name)}\n${trigger.timing} ${trigger.event} ON ${quoteIdentifier(tab.schema)}.${quoteIdentifier(tableName)}\nFOR EACH ROW\n${statement};`;
}

export function buildCreateTableSql(tab, state) {
  const tableName = normalizeSqlIdentifier(state.tableName);
  if (!tableName) {
    throw new Error("表名不能为空");
  }

  const activeColumns = state.columns.filter((column) => !column.dropped);
  if (activeColumns.length === 0) {
    throw new Error("至少需要一个字段");
  }

  const definitions = activeColumns.map(columnDefinition);
  const primaryColumns = activeColumns.filter((column) => column.primary).map((column) => quoteIdentifier(column.name));
  if (primaryColumns.length > 0) {
    definitions.push(`PRIMARY KEY (${primaryColumns.join(", ")})`);
  }
  for (const column of activeColumns.filter((item) => item.key === "UNIQUE")) {
    definitions.push(`UNIQUE KEY ${quoteIdentifier(`uk_${column.name}`)} (${quoteIdentifier(column.name)})`);
  }
  for (const column of activeColumns.filter((item) => item.key === "INDEX")) {
    definitions.push(`KEY ${quoteIdentifier(`idx_${column.name}`)} (${quoteIdentifier(column.name)})`);
  }
  for (const index of state.indexes.filter((item) => !item.dropped)) {
    definitions.push(indexDefinition(index));
  }
  for (const foreignKey of state.foreignKeys.filter((item) => !item.dropped)) {
    definitions.push(foreignKeyDefinition(foreignKey));
  }
  for (const check of state.checks.filter((item) => !item.dropped)) {
    definitions.push(checkDefinition(check));
  }

  const options = [
    state.engine ? `ENGINE=${state.engine}` : "",
    state.charset ? `DEFAULT CHARSET=${state.charset}` : "",
    state.collation ? `COLLATE=${state.collation}` : "",
    state.tableComment ? `COMMENT=${quoteString(state.tableComment)}` : "",
  ].filter(Boolean).join(" ");
  const statements = [`CREATE TABLE ${quoteIdentifier(tab.schema)}.${quoteIdentifier(tableName)} (\n  ${definitions.join(",\n  ")}\n) ${options};`];
  for (const trigger of state.triggers.filter((item) => !item.dropped)) {
    statements.push(triggerStatement({ ...tab, tableName }, trigger));
  }
  return statements;
}

function isColumnChanged(column) {
  const original = column.original;
  return !original
    || column.name !== original.name
    || column.typeName !== original.typeName
    || String(column.length ?? "") !== String(original.length ?? "")
    || String(column.scale ?? "") !== String(original.scale ?? "")
    || column.nullable !== original.nullable
    || column.virtual !== original.virtual
    || String(column.defaultValue ?? "") !== String(original.defaultValue ?? "")
    || column.autoIncrement !== original.autoIncrement
    || column.comment !== original.comment;
}

function isIndexChanged(index) {
  const original = index.original;
  return !original
    || index.name !== original.name
    || index.unique !== original.unique
    || String(index.columns ?? "") !== String(original.columns ?? "")
    || String(index.type ?? "") !== String(original.type ?? "");
}

function isForeignKeyChanged(foreignKey) {
  const original = foreignKey.original;
  return !original
    || foreignKey.name !== original.name
    || String(foreignKey.columns ?? "") !== String(original.columns ?? "")
    || foreignKey.referencedTable !== original.referencedTable
    || String(foreignKey.referencedColumns ?? "") !== String(original.referencedColumns ?? "")
    || foreignKey.onUpdate !== original.onUpdate
    || foreignKey.onDelete !== original.onDelete;
}

function isTriggerChanged(trigger) {
  const original = trigger.original;
  return !original
    || trigger.name !== original.name
    || trigger.timing !== original.timing
    || trigger.event !== original.event
    || String(trigger.statement ?? "") !== String(original.statement ?? "");
}

function isCheckChanged(check) {
  const original = check.original;
  return !original
    || check.name !== original.name
    || String(check.expression ?? "") !== String(original.expression ?? "")
    || check.enforced !== original.enforced;
}

function samePrimaryKey(state) {
  const before = state.columns
    .filter((column) => column.original?.primary)
    .map((column) => column.originalName)
    .join("\u0000");
  const after = state.columns
    .filter((column) => !column.dropped && column.primary)
    .map((column) => column.name)
    .join("\u0000");
  return before === after;
}

export function buildAlterTableSql(tab, state) {
  const tableName = normalizeSqlIdentifier(state.tableName);
  if (!tableName) {
    throw new Error("表名不能为空");
  }

  const clauses = [];
  if (tableName !== state.originalTable) {
    clauses.push(`RENAME TO ${quoteIdentifier(tab.schema)}.${quoteIdentifier(tableName)}`);
  }

  for (const column of state.columns) {
    if (column.original && column.dropped) {
      clauses.push(`DROP COLUMN ${quoteIdentifier(column.originalName)}`);
    } else if (!column.original && !column.dropped) {
      clauses.push(`ADD COLUMN ${columnDefinition(column)}`);
    } else if (column.original && !column.dropped && isColumnChanged(column)) {
      const keyword = column.name === column.originalName ? "MODIFY COLUMN" : `CHANGE COLUMN ${quoteIdentifier(column.originalName)}`;
      clauses.push(`${keyword} ${columnDefinition(column)}`);
    }
  }

  if (!samePrimaryKey(state)) {
    const hadPrimary = state.columns.some((column) => column.original?.primary);
    if (hadPrimary) {
      clauses.push("DROP PRIMARY KEY");
    }
    const primaryColumns = state.columns.filter((column) => !column.dropped && column.primary).map((column) => quoteIdentifier(column.name));
    if (primaryColumns.length > 0) {
      clauses.push(`ADD PRIMARY KEY (${primaryColumns.join(", ")})`);
    }
  }

  for (const index of state.indexes) {
    if (index.original && (index.dropped || isIndexChanged(index))) {
      clauses.push(`DROP INDEX ${quoteIdentifier(index.originalName)}`);
    }
    if (!index.dropped && (!index.original || isIndexChanged(index))) {
      clauses.push(`ADD ${indexDefinition(index)}`);
    }
  }

  for (const foreignKey of state.foreignKeys) {
    if (foreignKey.original && (foreignKey.dropped || isForeignKeyChanged(foreignKey))) {
      clauses.push(`DROP FOREIGN KEY ${quoteIdentifier(foreignKey.originalName)}`);
    }
    if (!foreignKey.dropped && (!foreignKey.original || isForeignKeyChanged(foreignKey))) {
      clauses.push(`ADD ${foreignKeyDefinition(foreignKey)}`);
    }
  }

  for (const check of state.checks) {
    if (check.original && (check.dropped || isCheckChanged(check))) {
      clauses.push(`DROP CHECK ${quoteIdentifier(check.originalName)}`);
    }
    if (!check.dropped && (!check.original || isCheckChanged(check))) {
      clauses.push(`ADD ${checkDefinition(check)}`);
    }
  }

  const originalOptions = state.originalOptions ?? {};
  if (state.engine && state.engine !== originalOptions.engine) {
    clauses.push(`ENGINE=${state.engine}`);
  }
  if (state.charset && state.charset !== originalOptions.charset) {
    clauses.push(`DEFAULT CHARACTER SET ${state.charset}`);
  }
  if (state.collation && state.collation !== originalOptions.collation) {
    clauses.push(`COLLATE ${state.collation}`);
  }
  if (String(state.tableComment ?? "") !== String(originalOptions.tableComment ?? "")) {
    clauses.push(`COMMENT=${quoteString(state.tableComment ?? "")}`);
  }

  const statements = [];
  for (const trigger of state.triggers) {
    if (trigger.original && (trigger.dropped || isTriggerChanged(trigger))) {
      statements.push(`DROP TRIGGER ${quoteIdentifier(tab.schema)}.${quoteIdentifier(trigger.originalName)};`);
    }
    if (!trigger.dropped && (!trigger.original || isTriggerChanged(trigger))) {
      statements.push(triggerStatement({ ...tab, tableName }, trigger));
    }
  }

  if (clauses.length > 0) {
    statements.unshift(`ALTER TABLE ${quoteIdentifier(tab.schema)}.${quoteIdentifier(state.originalTable)}\n  ${clauses.join(",\n  ")};`);
  }

  return statements;
}
