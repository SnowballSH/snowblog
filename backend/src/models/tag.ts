import { db } from "../utils/db.ts";
import { Row, QueryResult } from "../utils/db.ts";
import { v4 as uuid } from "uuid";

export interface Tag {
    id: string;
    name: string;
    count?: number; // Optional count of blogs using this tag
}

export class TagModel {
    /**
     * Get all tags with count of associated blogs
     */
    static getAll(): Tag[] {
        try {
            const tags: Tag[] = [];

            // Query all tags with count of associated blogs
            const tagRows = db.query(`
        SELECT t.id, t.name, COUNT(bt.blogId) as count
        FROM tags t
        LEFT JOIN blog_tags bt ON t.id = bt.tagId
        GROUP BY t.id
        ORDER BY count DESC, t.name ASC
      `) as QueryResult;

            for (const row of tagRows) {
                const id = row[0] as string;
                const name = row[1] as string;
                const count = row[2] as number;
                tags.push({ id, name, count });
            }

            return tags;
        } catch (error) {
            console.error("Error getting tags:", error);
            throw error;
        }
    }

    /**
     * Get a tag by ID
     */
    static getById(id: string): Tag | null {
        try {
            const tagRows = db.query(
                `SELECT t.id, t.name, COUNT(bt.blogId) as count
         FROM tags t
         LEFT JOIN blog_tags bt ON t.id = bt.tagId
         WHERE t.id = ?
         GROUP BY t.id`,
                [id]
            ) as QueryResult;

            if (!tagRows || tagRows.length === 0) {
                return null;
            }

            const row = tagRows[0];
            return {
                id: row[0] as string,
                name: row[1] as string,
                count: row[2] as number
            };
        } catch (error) {
            console.error("Error getting tag by ID:", error);
            throw error;
        }
    }

    /**
     * Create a new tag
     */
    static create(name: string): string {
        try {
            // Validate input
            if (!name || !name.trim()) {
                throw new Error("Tag name is required");
            }

            const trimmedName = name.trim();

            // Check if tag already exists
            const existingTagRows = db.query(
                `SELECT id FROM tags WHERE name = ?`,
                [trimmedName]
            ) as QueryResult;

            if (existingTagRows && existingTagRows.length > 0) {
                // Return existing tag ID
                return existingTagRows[0][0] as string;
            }

            // Create new tag
            const tagId = uuid();
            db.query(
                `INSERT INTO tags (id, name) VALUES (?, ?)`,
                [tagId, trimmedName]
            );

            return tagId;
        } catch (error) {
            console.error("Error creating tag:", error);
            throw error;
        }
    }

    /**
     * Update an existing tag
     */
    static update(id: string, name: string): boolean {
        try {
            // Validate input
            if (!id || !name || !name.trim()) {
                throw new Error("Tag ID and name are required");
            }

            const trimmedName = name.trim();

            // Check if tag exists
            const existingTagRows = db.query(
                `SELECT id FROM tags WHERE id = ?`,
                [id]
            ) as QueryResult;

            if (!existingTagRows || existingTagRows.length === 0) {
                return false;
            }

            // Update the tag
            db.query(
                `UPDATE tags SET name = ? WHERE id = ?`,
                [trimmedName, id]
            );

            return true;
        } catch (error) {
            console.error("Error updating tag:", error);
            throw error;
        }
    }

    /**
     * Delete a tag
     */
    static delete(id: string): boolean {
        try {
            // Check if tag exists
            const existingTagRows = db.query(
                `SELECT id FROM tags WHERE id = ?`,
                [id]
            ) as QueryResult;

            if (!existingTagRows || existingTagRows.length === 0) {
                return false;
            }

            // First delete associations in blog_tags
            db.query(
                `DELETE FROM blog_tags WHERE tagId = ?`,
                [id]
            );

            // Then delete the tag itself
            db.query(
                `DELETE FROM tags WHERE id = ?`,
                [id]
            );

            return true;
        } catch (error) {
            console.error("Error deleting tag:", error);
            throw error;
        }
    }

    /**
     * Get blogs associated with a tag
     */
    static getBlogsByTagName(tagName: string): string[] {
        try {
            const blogIds: string[] = [];

            // First get the tag ID
            const tagRows = db.query(
                `SELECT id FROM tags WHERE name = ?`,
                [tagName.trim()]
            ) as QueryResult;

            if (!tagRows || tagRows.length === 0) {
                return [];
            }

            const tagId = tagRows[0][0] as string;

            // Get all blog IDs associated with this tag
            const blogRows = db.query(`
        SELECT blogId
        FROM blog_tags
        WHERE tagId = ?
      `, [tagId]) as QueryResult;

            for (const row of blogRows) {
                const blogId = row[0] as string;
                blogIds.push(blogId);
            }

            return blogIds;
        } catch (error) {
            console.error("Error getting blogs by tag:", error);
            throw error;
        }
    }
}