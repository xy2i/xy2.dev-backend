CREATE TABLE comments
(
    id      SERIAL PRIMARY KEY,
    slug    VARCHAR(255)   NOT NULL,
    name    VARCHAR(255)   NOT NULL,
    date    TIMESTAMPTZ    NOT NULL DEFAULT now(),
    parent  INTEGER,
    text    VARCHAR(10000) NOT NULL,
    email   VARCHAR(255),
    visible BOOLEAN        NOT NULL DEFAULT false,
    CONSTRAINT fk_parent
        FOREIGN KEY (parent)
            REFERENCES comments (id)
);