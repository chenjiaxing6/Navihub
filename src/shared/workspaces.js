import {
  Connection,
  DataAnalysis,
} from "@element-plus/icons-vue";

export const workspaces = [
  { id: "database", label: "数据", title: "数据库", eyebrow: "工作区", icon: DataAnalysis, search: "搜索连接、库表、查询" },
  { id: "ssh", label: "终端", title: "终端/文件", eyebrow: "工作区", icon: Connection, search: "搜索 SSH 主机、路径、文件" },
];

export const workspaceActions = {
  database: ["新建查询", "导入数据", "备份"],
  ssh: ["新建终端", "上传文件", "端口转发"],
};
