import {
  assertEquals,
  assertExists,
  assertThrows,
} from "https://deno.land/std/testing/asserts.ts";
import { TagModel } from "./tag.ts";
import { BlogInput, BlogModel } from "./blog.ts";
import { db, useTestDb } from "../utils/db.ts";

// Helper to clean up the database after tests
function cleanDb() {
  db.query("DELETE FROM blog_tags");
  db.query("DELETE FROM blogs");
  db.query("DELETE FROM tags");
}

// Setup function to create a test tag
function createTestTag(name: string = "test-tag"): string {
  return TagModel.create(name);
}

// Setup function to create a test blog with tags
function createTestBlogWithTags(tags: string[]): string {
  const input: BlogInput = {
    title: "Test Blog",
    content: "This is test content for the blog",
    tags: tags,
  };
  return BlogModel.create(input);
}

// Run before all tests
Deno.test({
  name: "Setup test environment for tags",
  fn: () => {
    useTestDb();
    cleanDb();
  },
});

// Test creating a tag
Deno.test("TagModel.create - should create a new tag", () => {
  cleanDb();
  const tagId = createTestTag();

  assertExists(tagId);

  const tag = TagModel.getById(tagId);
  assertExists(tag);
  assertEquals(tag?.name, "test-tag");
});

Deno.test("TagModel.create - should return existing tag id if tag exists", () => {
  cleanDb();
  const tagId1 = TagModel.create("duplicate-tag");
  const tagId2 = TagModel.create("duplicate-tag");

  assertEquals(tagId1, tagId2);
});

Deno.test("TagModel.create - should throw error for empty tag name", () => {
  assertThrows(
    () => {
      TagModel.create("");
    },
    Error,
    "Tag name is required",
  );
});

// Test getting all tags
Deno.test("TagModel.getAll - should return all tags with counts", () => {
  cleanDb();
  const tagId = createTestTag();
  createTestBlogWithTags(["test-tag"]);

  const tags = TagModel.getAll();

  assertEquals(tags.length > 0, true);
  const testTag = tags.find((t) => t.id === tagId);
  assertExists(testTag);
  assertEquals(testTag.name, "test-tag");
  assertEquals(testTag.count, 1);
});

// Test getting a tag by ID
Deno.test("TagModel.getById - should return a tag by ID", () => {
  cleanDb();
  const tagId = createTestTag();

  const tag = TagModel.getById(tagId);

  assertExists(tag);
  assertEquals(tag?.id, tagId);
  assertEquals(tag?.name, "test-tag");
});

Deno.test("TagModel.getById - should return null for non-existent tag", () => {
  const tag = TagModel.getById("non-existent-id");
  assertEquals(tag, null);
});

// Test updating a tag
Deno.test("TagModel.update - should update a tag", () => {
  cleanDb();
  const tagId = createTestTag();

  const result = TagModel.update(tagId, "updated-tag");
  assertEquals(result, true);

  const updatedTag = TagModel.getById(tagId);
  assertEquals(updatedTag?.name, "updated-tag");
});

Deno.test("TagModel.update - should return false for non-existent tag", () => {
  const result = TagModel.update("non-existent-id", "new-name");
  assertEquals(result, false);
});

Deno.test("TagModel.update - should throw error for empty tag name", () => {
  const tagId = createTestTag();

  assertThrows(
    () => {
      TagModel.update(tagId, "");
    },
    Error,
    "Tag ID and name are required",
  );
});

// Test deleting a tag
Deno.test("TagModel.delete - should delete a tag", () => {
  cleanDb();
  const tagId = createTestTag();

  const result = TagModel.delete(tagId);
  assertEquals(result, true);

  const tag = TagModel.getById(tagId);
  assertEquals(tag, null);
});

Deno.test("TagModel.delete - should return false for non-existent tag", () => {
  const result = TagModel.delete("non-existent-id");
  assertEquals(result, false);
});

Deno.test("TagModel.delete - should remove tag associations when deleting", () => {
  cleanDb();
  createTestBlogWithTags(["tag-to-delete"]);

  // Find the tag ID
  const tags = TagModel.getAll();
  const tagToDelete = tags.find((t) => t.name === "tag-to-delete");
  assertExists(tagToDelete);

  // Delete the tag
  const result = TagModel.delete(tagToDelete.id);
  assertEquals(result, true);

  // Verify tag associations are gone
  const blogsByTag = TagModel.getBlogsByTagName("tag-to-delete");
  assertEquals(blogsByTag.length, 0);
});

// Test getting blogs by tag name
Deno.test("TagModel.getBlogsByTagName - should return blog IDs with specific tag", () => {
  cleanDb();
  const blogId = createTestBlogWithTags(["special-tag"]);

  const blogIds = TagModel.getBlogsByTagName("special-tag");

  assertEquals(blogIds.length, 1);
  assertEquals(blogIds[0], blogId);
});

Deno.test("TagModel.getBlogsByTagName - should return empty array for non-existent tag", () => {
  const blogIds = TagModel.getBlogsByTagName("non-existent-tag");
  assertEquals(blogIds.length, 0);
});

// Test tag sorting and ordering
Deno.test("TagModel.getAll - should sort tags by count and name", () => {
  cleanDb();

  // Create tags with different usage counts
  const tag1 = createTestTag("popular-tag");
  const tag2 = createTestTag("average-tag");
  const tag3 = createTestTag("unused-tag");

  // Create blogs with these tags
  createTestBlogWithTags(["popular-tag"]);
  createTestBlogWithTags(["popular-tag"]);
  createTestBlogWithTags(["average-tag"]);

  const tags = TagModel.getAll();

  // Popular tag should come first (count=2)
  assertEquals(tags[0].name, "popular-tag");
  assertEquals(tags[0].count, 2);

  // Average tag should come next (count=1)
  assertEquals(tags[1].name, "average-tag");
  assertEquals(tags[1].count, 1);

  // Unused tag should come last (count=0)
  assertEquals(tags[2].name, "unused-tag");
  assertEquals(tags[2].count, 0);
});

// Cleanup after all tests
Deno.test({
  name: "Cleanup test environment for tags",
  fn: () => {
    cleanDb();
  },
});
