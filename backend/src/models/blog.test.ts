import {
  assertEquals,
  assertExists,
  assertThrows,
} from "https://deno.land/std/testing/asserts.ts";
import { BlogInput, BlogModel } from "./blog.ts";
import { db, useTestDb } from "../utils/db.ts";

// Helper to clean up the database after tests
function cleanDb() {
  db.query("DELETE FROM blog_tags");
  db.query("DELETE FROM blogs");
  db.query("DELETE FROM tags");
}

// Setup function to create a test blog
function createTestBlog(): string {
  const input: BlogInput = {
    title: "Test Blog",
    content: "This is test content for the blog",
    tags: ["test", "unit-testing"],
  };
  return BlogModel.create(input);
}

// Run before all tests
Deno.test({
  name: "Setup test environment",
  fn: () => {
    useTestDb();
    cleanDb();
  },
});

// Test creating a blog
Deno.test("BlogModel.create - should create a blog post", () => {
  const input: BlogInput = {
    title: "Test Blog Creation",
    content: "This is a test blog post content",
    tags: ["test", "creation"],
  };

  const blogId = BlogModel.create(input);
  assertExists(blogId);

  const blog = BlogModel.getById(blogId);
  assertEquals(blog?.title, input.title);
  assertEquals(blog?.content, input.content);
  assertEquals(blog?.tags.sort(), (input.tags ?? []).sort());
});

Deno.test("BlogModel.create - should throw error for missing required fields", () => {
  assertThrows(
    () => {
      BlogModel.create({ title: "", content: "" });
    },
    Error,
    "Title and content are required",
  );
});

// Test getting all blog summaries
Deno.test("BlogModel.getAllSummaries - should return blog summaries", () => {
  cleanDb();
  const blogId = createTestBlog();
  const summaries = BlogModel.getAllSummaries();

  assertEquals(summaries.length > 0, true);
  assertEquals(summaries[0].id, blogId);
  assertEquals(summaries[0].title, "Test Blog");
  assertExists(summaries[0].excerpt);
  assertEquals(summaries[0].tags.includes("test"), true);
});

// Test getting a blog by ID
Deno.test("BlogModel.getById - should return a blog by ID", () => {
  const blogId = createTestBlog();
  const blog = BlogModel.getById(blogId);

  assertExists(blog);
  assertEquals(blog?.id, blogId);
  assertEquals(blog?.title, "Test Blog");
  assertEquals(blog?.content, "This is test content for the blog");
  assertEquals(blog?.tags.includes("test"), true);
});

Deno.test("BlogModel.getById - should return null for non-existent blog", () => {
  const blog = BlogModel.getById("non-existent-id");
  assertEquals(blog, null);
});

// Test updating a blog
Deno.test("BlogModel.update - should update a blog post", () => {
  const blogId = createTestBlog();

  const updateInput: Partial<BlogInput> = {
    title: "Updated Title",
    tags: ["updated", "tag"],
  };

  const result = BlogModel.update(blogId, updateInput);
  assertEquals(result, true);

  const updatedBlog = BlogModel.getById(blogId);
  assertEquals(updatedBlog?.title, "Updated Title");
  assertEquals(updatedBlog?.content, "This is test content for the blog");
  assertEquals(updatedBlog?.tags.includes("updated"), true);
  assertEquals(updatedBlog?.tags.includes("test"), false);
});

Deno.test("BlogModel.update - should return false for non-existent blog", () => {
  const result = BlogModel.update("non-existent-id", { title: "New Title" });
  assertEquals(result, false);
});

// Test deleting a blog
Deno.test("BlogModel.delete - should delete a blog post", () => {
  const blogId = createTestBlog();

  const result = BlogModel.delete(blogId);
  assertEquals(result, true);

  const blog = BlogModel.getById(blogId);
  assertEquals(blog, null);
});

Deno.test("BlogModel.delete - should return false for non-existent blog", () => {
  const result = BlogModel.delete("non-existent-id");
  assertEquals(result, false);
});

// Test searching blogs
Deno.test("BlogModel.search - should find blogs by keyword", () => {
  cleanDb();
  createTestBlog();

  const searchResults = BlogModel.search("test");
  assertEquals(searchResults.length > 0, true);
  assertEquals(
    searchResults[0].title.includes("Test") ||
      searchResults[0].excerpt.includes("test"),
    true,
  );
});

// Test getting blogs by tag
Deno.test("BlogModel.getByTag - should get blogs by tag", () => {
  cleanDb();
  createTestBlog();

  const taggedBlogs = BlogModel.getByTag("test");
  assertEquals(taggedBlogs.length > 0, true);
  assertEquals(taggedBlogs[0].tags.includes("test"), true);
});

// Test counting blogs
Deno.test("BlogModel.count - should count the total blogs", () => {
  cleanDb();
  createTestBlog();
  createTestBlog();

  const count = BlogModel.count();
  assertEquals(count, 2);
});

// Cleanup after all tests
Deno.test({
  name: "Cleanup test environment",
  fn: () => {
    cleanDb();
  },
});
