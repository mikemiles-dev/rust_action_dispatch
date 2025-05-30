document.addEventListener('DOMContentLoaded', function() {
    // Select all elements with the 'utc-date' class
    const dateCells = document.querySelectorAll('.utc-date');

    dateCells.forEach(cell => {
        const timestamp = parseInt(cell.dataset.timestamp); // Get the timestamp from data-timestamp
        if (!isNaN(timestamp)) {
            const date = new Date(timestamp); // Create a Date object from the milliseconds timestamp

            // Format it as a UTC string
            // Option 1: Standard UTC string
            // const utcString = date.toUTCString();

            // Option 2: ISO 8601 UTC string (often preferred for consistency)
            // const utcString = date.toISOString();

            // Option 3: More readable custom UTC format using Intl.DateTimeFormat
            const options = {
                year: 'numeric',
                month: 'short',
                day: 'numeric',
                hour: '2-digit',
                minute: '2-digit',
                second: '2-digit',
                hour12: false, // Use 24-hour format
                timeZone: 'UTC', // Ensure it's UTC
                timeZoneName: 'short' // e.g., "GMT" or "UTC"
            };
            const utcString = new Intl.DateTimeFormat('en-US', options).format(date);


            cell.textContent = utcString; // Replace the cell's content with the formatted date
        }
    });
});
