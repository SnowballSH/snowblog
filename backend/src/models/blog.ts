import { db } from "../utils/db.ts";
import { QueryResult, Row } from "../utils/db.ts";
import { v4 as uuid } from "uuid";

// Define Blog interfaces
export interface BlogInput {
  title: string;
  content: string;
  tags?: string[];
}

export interface Blog {
  id: string;
  title: string;
  content: string;
  createdAt: string;
  tags: string[];
}

export interface BlogSummary {
  id: string;
  title: string;
  excerpt: string;
  createdAt: string;
  tags: string[];
}

export class BlogModel {
  /**
   * Create a new blog post
   */
  static create(input: BlogInput): string {
    try {
      // Validate input
      if (!input.title || !input.content) {
        throw new Error("Title and content are required");
      }

      // Generate a new UUID for the blog
      const blogId = uuid();

      // Insert the blog
      db.query(
        `INSERT INTO blogs (id, title, content) VALUES (?, ?, ?)`,
        [blogId, input.title, input.content],
      );

      // Handle tags if provided
      if (input.tags && Array.isArray(input.tags) && input.tags.length > 0) {
        for (const tagName of input.tags) {
          // Skip empty tags
          if (!tagName.trim()) continue;

          // Check if tag exists, if not create it
          let tagId: string;
          const existingTagRows = db.query(
            `SELECT id FROM tags WHERE name = ?`,
            [tagName.trim()],
          ) as QueryResult;

          if (existingTagRows && existingTagRows.length > 0) {
            tagId = existingTagRows[0][0] as string;
          } else {
            // Create new tag
            tagId = uuid();
            db.query(
              `INSERT INTO tags (id, name) VALUES (?, ?)`,
              [tagId, tagName.trim()],
            );
          }

          // Associate tag with blog
          db.query(
            `INSERT INTO blog_tags (blogId, tagId) VALUES (?, ?)`,
            [blogId, tagId],
          );
        }
      }

      return blogId;
    } catch (error) {
      console.error("Error creating blog:", error);
      throw error;
    }
  }

  /**
   * Get all blogs with summaries
   */
  static getAllSummaries(): BlogSummary[] {
    try {
      const blogs: BlogSummary[] = [];

      // Get all blogs ordered by creation date (newest first)
      const blogRows = db.query(`
                SELECT id, title, content, createdAt
                FROM blogs
                ORDER BY createdAt DESC
            `) as QueryResult;

      for (const row of blogRows) {
        const [id, title, content, createdAt] = row as [
          string,
          string,
          string,
          string,
        ];

        // Get tags for this blog
        const tags: string[] = [];
        const tagRows = db.query(
          `
                    SELECT t.name
                    FROM tags t
                    JOIN blog_tags bt ON t.id = bt.tagId
                    WHERE bt.blogId = ?
                `,
          [id],
        ) as QueryResult;

        for (const tagRow of tagRows) {
          const tagName = tagRow[0] as string;
          tags.push(tagName);
        }

        // Create an excerpt from content (first 150 chars)
        const excerpt = content.length > 150
          ? content.substring(0, 150) + "..."
          : content;

        blogs.push({
          id,
          title,
          excerpt,
          createdAt,
          tags,
        });
      }

      return blogs;
    } catch (error) {
      console.error("Error getting blog summaries:", error);
      throw error;
    }
  }

  /**
   * Get a single blog by ID
   */
  static getById(id: string): Blog | null {
    try {
      // Get the blog
      const blogRows = db.query(
        `
                SELECT id, title, content, createdAt
                FROM blogs
                WHERE id = ?
            `,
        [id],
      ) as QueryResult;

      if (!blogRows || blogRows.length === 0) {
        return null;
      }

      const [blogId, title, content, createdAt] = blogRows[0] as [
        string,
        string,
        string,
        string,
      ];

      // Get tags for this blog
      const tags: string[] = [];
      const tagRows = db.query(
        `
                SELECT t.name
                FROM tags t
                JOIN blog_tags bt ON t.id = bt.tagId
                WHERE bt.blogId = ?
            `,
        [id],
      ) as QueryResult;

      for (const tagRow of tagRows) {
        const tagName = tagRow[0] as string;
        tags.push(tagName);
      }

      return {
        id: blogId,
        title,
        content,
        createdAt,
        tags,
      };
    } catch (error) {
      console.error("Error getting blog by ID:", error);
      throw error;
    }
  }

  /**
   * Update an existing blog post
   */
  static update(id: string, input: Partial<BlogInput>): boolean {
    try {
      // Check if blog exists
      const blogExists = db.query(
        `SELECT id FROM blogs WHERE id = ?`,
        [id],
      ) as QueryResult;

      if (!blogExists || blogExists.length === 0) {
        return false;
      }

      // Update blog fields if provided
      if (input.title || input.content) {
        let query = "UPDATE blogs SET ";
        const params: any[] = [];

        if (input.title) {
          query += "title = ?";
          params.push(input.title);

          if (input.content) {
            query += ", content = ?";
            params.push(input.content);
          }
        } else if (input.content) {
          query += "content = ?";
          params.push(input.content);
        }

        query += " WHERE id = ?";
        params.push(id);

        db.query(query, params);
      }

      // Update tags if provided
      if (input.tags && Array.isArray(input.tags)) {
        // Remove existing tag associations
        db.query(`DELETE FROM blog_tags WHERE blogId = ?`, [id]);

        // Add new tag associations
        for (const tagName of input.tags) {
          if (!tagName.trim()) continue;

          // Check if tag exists, if not create it
          let tagId: string;
          const existingTagRows = db.query(
            `SELECT id FROM tags WHERE name = ?`,
            [tagName.trim()],
          ) as QueryResult;

          if (existingTagRows && existingTagRows.length > 0) {
            tagId = existingTagRows[0][0] as string;
          } else {
            // Create new tag
            tagId = uuid();
            db.query(
              `INSERT INTO tags (id, name) VALUES (?, ?)`,
              [tagId, tagName.trim()],
            );
          }

          // Associate tag with blog
          db.query(
            `INSERT INTO blog_tags (blogId, tagId) VALUES (?, ?)`,
            [id, tagId],
          );
        }
      }

      return true;
    } catch (error) {
      console.error("Error updating blog:", error);
      throw error;
    }
  }

  /**
   * Delete a blog post
   */
  static delete(id: string): boolean {
    try {
      // Check if blog exists
      const blogExists = db.query(
        `SELECT id FROM blogs WHERE id = ?`,
        [id],
      ) as QueryResult;

      if (!blogExists || blogExists.length === 0) {
        return false;
      }

      // Delete tag associations first (handle foreign key constraints)
      db.query(`DELETE FROM blog_tags WHERE blogId = ?`, [id]);

      // Delete the blog
      db.query(`DELETE FROM blogs WHERE id = ?`, [id]);

      return true;
    } catch (error) {
      console.error("Error deleting blog:", error);
      throw error;
    }
  }

  /**
   * Search blogs by keyword in title or content
   */
  static search(keyword: string): BlogSummary[] {
    try {
      const blogs: BlogSummary[] = [];
      const searchTerm = `%${keyword}%`;

      // Search in title and content
      const blogRows = db.query(
        `
                SELECT id, title, content, createdAt
                FROM blogs
                WHERE title LIKE ? OR content LIKE ?
                ORDER BY createdAt DESC
            `,
        [searchTerm, searchTerm],
      ) as QueryResult;

      for (const row of blogRows) {
        const [id, title, content, createdAt] = row as [
          string,
          string,
          string,
          string,
        ];

        // Get tags for this blog
        const tags: string[] = [];
        const tagRows = db.query(
          `
                    SELECT t.name
                    FROM tags t
                    JOIN blog_tags bt ON t.id = bt.tagId
                    WHERE bt.blogId = ?
                `,
          [id],
        ) as QueryResult;

        for (const tagRow of tagRows) {
          const tagName = tagRow[0] as string;
          tags.push(tagName);
        }

        // Create an excerpt from content (first 150 chars)
        const excerpt = content.length > 150
          ? content.substring(0, 150) + "..."
          : content;

        blogs.push({
          id,
          title,
          excerpt,
          createdAt,
          tags,
        });
      }

      return blogs;
    } catch (error) {
      console.error("Error searching blogs:", error);
      throw error;
    }
  }

  /**
   * Get blogs by tag
   */
  static getByTag(tagName: string): BlogSummary[] {
    try {
      const blogs: BlogSummary[] = [];

      // Get blogs with the specified tag
      const blogRows = db.query(
        `
                SELECT b.id, b.title, b.content, b.createdAt
                FROM blogs b
                JOIN blog_tags bt ON b.id = bt.blogId
                JOIN tags t ON bt.tagId = t.id
                WHERE t.name = ?
                ORDER BY b.createdAt DESC
            `,
        [tagName],
      ) as QueryResult;

      for (const row of blogRows) {
        const [id, title, content, createdAt] = row as [
          string,
          string,
          string,
          string,
        ];

        // Get all tags for this blog
        const tags: string[] = [];
        const tagRows = db.query(
          `
                    SELECT t.name
                    FROM tags t
                    JOIN blog_tags bt ON t.id = bt.tagId
                    WHERE bt.blogId = ?
                `,
          [id],
        ) as QueryResult;

        for (const tagRow of tagRows) {
          const tagName = tagRow[0] as string;
          tags.push(tagName);
        }

        // Create an excerpt from content
        const excerpt = content.length > 150
          ? content.substring(0, 150) + "..."
          : content;

        blogs.push({
          id,
          title,
          excerpt,
          createdAt,
          tags,
        });
      }

      return blogs;
    } catch (error) {
      console.error("Error getting blogs by tag:", error);
      throw error;
    }
  }

  /**
   * Count total blogs (useful for pagination)
   */
  static count(): number {
    try {
      const result = db.query(`SELECT COUNT(*) FROM blogs`) as QueryResult;
      return result[0][0] as number;
    } catch (error) {
      console.error("Error counting blogs:", error);
      throw error;
    }
  }
}
