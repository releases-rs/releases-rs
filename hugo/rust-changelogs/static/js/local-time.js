
// Convert UTC timestamps to local time
document.addEventListener('DOMContentLoaded', function() {
    document.querySelectorAll('.utc-timestamp').forEach(function(element) {
        const utcTimestamp = element.getAttribute('data-utc');

        try {
            const utcDate = new Date(utcTimestamp);
            
            element.innerHTML = `${utcDate.toLocaleString()}`;
            
            element.setAttribute('title', `UTC: ${utcDate.toUTCString()}`);
            
            element.style.cursor = 'help';
            element.style.borderBottom = '1px dotted #666';
        } catch (error) {
            console.warn('Failed to parse UTC timestamp:', utcTimestamp, error);
        }
    });
});
