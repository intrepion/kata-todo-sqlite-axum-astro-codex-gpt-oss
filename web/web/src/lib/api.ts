import axios from "axios";
import type { Task } from "./types";

const api = axios.create({ baseURL: "/api" });

export const getTasks = async (): Promise<Task[]> => {
  const { data } = await api.get("/tasks");
  return data;
};

export const createTask = async (title: string, desc?: string): Promise<Task> => {
  const { data } = await api.post("/tasks", { title, description: desc });
  return data;
};

export const updateTask = async (id: number, updates: Partial<Task>): Promise<Task> => {
  const { data } = await api.put(`/tasks/${id}`, updates);
  return data;
};

export const deleteTask = async (id: number): Promise<void> => {
  await api.delete(`/tasks/${id}`);
};
