export const connections = [
  {
    id: "prod",
    workspace: "database",
    name: "prod-mysql-01",
    meta: "MySQL · 10.8.12.24",
    iconClass: "mysql",
    iconText: "M",
    config: {
      host: "127.0.0.1",
      port: 3306,
      username: "root",
      password: "",
      database: "employees",
    },
    schemas: [
      {
        name: "employees",
        groups: [
          { type: "table", title: "表", count: 6, items: ["departments", "dept_emp", "dept_manager", "employees", "salaries", "titles"] },
          { type: "view", title: "视图", count: 1, items: ["active_employees"] },
          { type: "query", title: "查询", count: 1, items: ["recent_salary_changes"] },
          { type: "function", title: "存储过程/函数", count: 1, items: ["current_manager"] },
        ],
      },
      { name: "fgdfg", groups: [] },
      { name: "hexhub", groups: [] },
    ],
  },
  {
    id: "analytics",
    workspace: "database",
    name: "analytics-pg",
    meta: "PostgreSQL · bastion 转发",
    iconClass: "postgres",
    iconText: "P",
    schemas: [
      {
        name: "warehouse",
        groups: [
          { type: "table", title: "表", count: 4, items: ["events", "funnels", "retention_daily", "segments"] },
          { type: "view", title: "视图", count: 1, items: ["revenue_by_channel"] },
          { type: "query", title: "查询", count: 1, items: ["weekly_growth_report"] },
          { type: "function", title: "存储过程/函数", count: 1, items: ["refresh_rollups"] },
        ],
      },
    ],
  },
];
