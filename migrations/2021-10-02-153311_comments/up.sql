CREATE TABLE comments
(
    id     SERIAL PRIMARY KEY,
    slug   VARCHAR(255)              NOT NULL,
    name   VARCHAR(255)              NOT NULL,
    date   TIMESTAMPTZ DEFAULT Now() NOT NULL,
    parent INTEGER,
    text   VARCHAR(10000)            NOT NULL,
    email  VARCHAR(255),
    CONSTRAINT fk_parent
        FOREIGN KEY (parent)
            REFERENCES comments (id)
);