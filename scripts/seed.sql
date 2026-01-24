-- Development Database Seeding Script
-- Run with: psql -U aircade -d aircade -f scripts/seed.sql

-- Clear existing data (in reverse order of foreign key dependencies)
DELETE FROM players;
DELETE FROM games;
DELETE FROM users;

-- Reset sequences
ALTER SEQUENCE users_id_seq RESTART WITH 1;
ALTER SEQUENCE games_id_seq RESTART WITH 1;
ALTER SEQUENCE players_id_seq RESTART WITH 1;

-- Insert test users
INSERT INTO users (username, created_at, updated_at) VALUES
('alice', NOW(), NOW()),
('bob', NOW(), NOW()),
('charlie', NOW(), NOW()),
('diana', NOW(), NOW()),
('eve', NOW(), NOW());

-- Insert test games
INSERT INTO games (code, host_id, status, settings, created_at) VALUES
('ABC123', 1, 'lobby', '{"maxPlayers": 8, "gameType": "quickdraw"}', NOW()),
('XYZ789', 2, 'playing', '{"maxPlayers": 6, "gameType": "trivia"}', NOW()),
('DEF456', 3, 'finished', '{"maxPlayers": 4, "gameType": "reaction"}', NOW());

-- Insert test players
INSERT INTO players (game_id, user_id, nickname, color, joined_at) VALUES
(1, 1, 'Alice', '#FF5733', NOW()),
(1, 2, 'Bob', '#33FF57', NOW()),
(1, 3, 'Charlie', '#3357FF', NOW()),
(2, 2, 'Bob', '#FFD700', NOW()),
(2, 4, 'Diana', '#FF1493', NOW()),
(3, 3, 'Charlie', '#00CED1', NOW()),
(3, 5, 'Eve', '#FF69B4', NOW());

-- Display seeded data
SELECT 'Users:' AS table_name;
SELECT * FROM users;

SELECT 'Games:' AS table_name;
SELECT * FROM games;

SELECT 'Players:' AS table_name;
SELECT * FROM players;
