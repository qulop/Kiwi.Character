# Main idea

The application is the Character.AI analogue, but for usage with local AI models from LM Studio or Ollama.

The application would consists from **two pages**: 
+ *Main page* - the page where user can pick characters to talk with
+ *Chat page* - the page where user will chat with chosen character

# Application style

+ Colors: dark colors(black, grey, white(for lines))
+ Accent colors: purple
+ Font: ElmsSans
 

# Main page

Layout of this page will be divided on two parts(from left to right):
+ Characters chat history part
+ Characters pickup part 


## Characters pickup part - Main part, ~3/4 size of the window size

Here user should see:
+ List of all available characters to chat with. 

The list should contain buttons with such layout:
+ Character profile image. The image should has form of "squircle"(same as `corner-shape: superellipse(2);` in CSS)
+ Character info(from top to bottom):
    + Character name
    + Short character info 

## Characters chat history part - Left sidebar, ~1/4 size of the window size

Here user should see(from top to bottom):
+ The application logo
+ "Create" button - button for a new character creation. Rounded button with big plus icon on it's left side(like "+ Create")
+ "Search" field, where the user can search throuth the chat history
+ List of all characters, which the users was chat before. The list should contain "character buttons"
+ Settings button

Each "character buttons" consists of three main part(from left to right):
+ Character profile image on the left side of the button. The image should be rounded with 360(same as `border-radius: 50%;` in CSS)
+ Character name
+ "Three dots" button, which is displaying only when cursor is hovering the main button

After the user clicks "Create" button, the entire page should be darkened(~for about 70%) and a pop-up window should appear in center

Settings button should be a small "gear" button. After the user clicks this button, the entire page should be darkened(~for about 70%) and a pop-up window should appear in center

## Settings pop-up window - In center, ~50% of the window width and ~60% of the window height

The settings popup is a modal window that overlays the main interface. It maintains the global dark theme of the application.

### Window States

The popup has two distinct interactive states:
    + Unverified Endpoint (Default): Only the header and the "Endpoint Configuration" block are active. All content below it is visually dimmed (e.g., using opacity: 0.4; or a dark overlay) and set to a disabled state to prevent interaction.
    + Verified Endpoint: Triggered when the app successfully pings the endpoint. The dimming is removed, and all model configuration settings become fully visible and interactive.

### Window Structure (from top to bottom)
#### 1. Header

The top bar of the popup. Items are arranged horizontally:
    + Settings Title: An icon (e.g., a gear) paired with the text "Settings" on the left side.
    + Close Button: A simple "X" button on the far right side to close the modal.
    + Divider: A thin horizontal line(with white color) spanning the full width to separate the header from the main content.

#### 2. Endpoint Configuration

This block is always active. Elements are placed in a single horizontal row (from left to right):
    + Label: Text "API Endpoint:".
    + Input Field: A text field where the user specifies the local server address. Default value (placeholder): "http://localhost:1234/v1".
    + "Test" Button: A button placed next to the input field to verify the connection.

#### --- Horizontal Line Divider(with white color) ---

#### 3. Model Configuration

(This entire section remains dimmed and disabled until the Endpoint is successfully verified).

+ Model Selection:
    + A dropdown list containing all available AI models fetched from the verified endpoint.
+ Model Settings:
    + A structured block containing parameters for the selected model:
        + Context Length: A slider control. Default value is set to 100k.
        + GPU Offload: A slider control to adjust how many model layers are offloaded to the GPU (from 0 to Max).
    + Advanced Settings(Can be grouped inside an expandable "Advanced" section or listed below the sliders):
        +Temperature: A slider ranging from 0.0 to 2.0 to control the creativity/randomness of the model's responses.
        +Max Tokens: A numeric input field to limit the maximum length of the generated response.
        +System Prompt: A text area field to set a default hidden prompt that dictates the overarching behavior of the AI.

#### 4. Action Area (Footer)
    + "Load model" Button: A primary action button located at the bottom of the popup (right-aligned). Clicking it initiates the model loading process on the local server.



## "New Character" Popup - In center, ~50% of the window width and ~60% of the window height

This modal window appears when the user clicks the "Create" button in the Left Sidebar. It is designed to capture all the necessary persona data to feed into the local LLM.

### Window Dimensions & Position
    + Scroll Behavior: Since the form contains multiple tall text areas, the content inside the modal should be scrollable.

### Window Structure (from top to bottom)
#### 1. Header

The top bar of the popup. Items are arranged horizontally (from left to right):
    + Window Icon: A contextual icon (e.g., a person with a "+" sign or a user profile silhouette).
    + Title: Label "New Character".
    + Close Button: A simple "X" button on the far right side to close the modal.

#### --- Horizontal Line Divider(with white color) ---
#### 2. Character Avatar

Positioned horizontally in the center of the window:
    + Interactive Avatar Area: A circular button/container.
    + Empty State: Displays a placeholder avatar (e.g., a neutral silhouette or camera icon) until the user uploads an image.
    + Filled State: Displays the user's uploaded image, strictly cropped to a circle.

#### 3. Character Details (Form Fields)

A vertical stack of input fields. Each block consists of a label and an input element (from top to bottom):
+ Name:
    + Label: "Character Name"
    + Input: Standard single-line text field.
    + Placeholder: "Example: John Doe"
+ Short Info(Note: This text will be displayed under the character's name on the Main Page pickup cards):
    + Label: "Short info"
    + Input: Standard single-line text field.
    + Placeholder: "Example: Mysterious man with kind heart"
+ Appearance:
    + Label: "Character Appearance Description"
    + Input: A tall, multi-line text area.
    + Placeholder: "Example: A tall man, usually wearing a cloak with fedora hat"
+ Overall Description:
    + Label: "Overall Description"
    + Input: A tall, multi-line text area.
    + Placeholder: "Example: A man with kind soul. Likes chocolate and hats"
+ Greeting Message(Note: This is the initial message that starts the chat):
    + Label: "Initial Chat Message"
    + Input: A tall, multi-line text area.
    + Placeholder: "Example: Hello! My name is John! Who are you, stranger?"

#### 4. Action Area (Footer)
    + "Create Character" Button: A primary action button located at the very bottom of the modal.
        + Styling: Highly rounded corners (e.g., `border-radius: 30%;` or pill-shaped).



# Chat page

Layout of this page will be divided on three parts(from left ot right):
+ Characters chat history part
+ Chat part
+ Chat settings part


## Characters chat history part - Left sidebar, ~1/4 size of the window size

Design of this part should be **absolutely same** as on the *main page*


## Chat part - In center, ~1/2 size of the window width

This is the primary interaction area where the conversation takes place.

### Layout (from top to bottom)
    + Chat Messages Area: A large, scrollable container displaying the active conversation.
        + AI Character messages are positioned on the left side of the screen.
        + User messages are positioned on the right side of the screen.
    + Input Area: Pinned to the bottom of the section. Arranged horizontally (from left to right):
        + Message Input Field: A text field with rounded corners for typing messages.
        + Send Button: A rounded button placed next to the input field.

### Message Bubble Design

Each message block maintains a consistent internal structure:
    + Avatar (Left side of the message block):
        + User: Displays the selected "Persona" avatar. If no persona is selected, displays a default `UserDefaultIcon`.
        + Character: Displays the character's circular avatar.
    + Message Content (Placed to the right of the avatar, top to bottom):
        + Title: Displays the Character's name or the User's Persona name. If no persona is selected, it simply displays "User".
        + Message Bubble: + Background color: #1c1f20.
            + Text color: White.


## Chat settings part - Right sidebar, ~1/4 size of the window width

This section is dedicated to managing the user's personas and viewing the current character's details.

### Layout (from top to bottom)
#### 1. Character Information

Displays the details of the character currently in the chat.
    + Identity Row (arranged horizontally, from left to right):
        + Character Avatar: A circular profile image of the character.
        + Character Name: A title displaying the character's name.
    + Details (arranged vertically below the identity row):
        + Short Info: A text block displaying the "Short character info" associated with the character.

#### --- Horizontal Line Divider ---(white color)
#### 2. Personas Management
    + "Personas" Button: A rounded button with the title "Personas".
    + State: Currently set to inactive/disabled (serves as a placeholder for future persona selection functionality).